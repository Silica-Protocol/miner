Got it—let’s keep it “real” by grounding everything in BOINC’s own FLOPs math, host benchmarks, live progress, and completed-task deltas. Here’s a clean, reproducible pipeline you can implement.

0) Where to read things (per OS)

BOINC data directory (default)

Linux: /var/lib/boinc-client/

Windows: C:\ProgramData\BOINC\

macOS: /Library/Application Support/BOINC Data/


Key files / interfaces

client_state.xml — canonical source for host benchmarks, workunits/results metadata (incl. rsc_fpops_est, rsc_fpops_bound, app version/plan class).

client_state_prev.xml — fallback if client_state.xml mid-write is inconsistent.

client_statistics.xml — long-run host stats (optional).

sched_reply_*.xml (per project) — sometimes carries helpful app/version hints (optional).

slots/<N>/ — active task runtime files (optional; do not rely on file scraping for live progress).

gui_rpc_auth.cfg — token for GUI RPC.

GUI RPC (TCP 31416) — use this for live: get_state, get_tasks, get_host_info.


> You’ll combine static (XML) and live (RPC) data: XML gives the size of the jobs in FLOPs; RPC gives fraction done and time.




---

1) Build a local inventory at startup (read once, refresh if file mtime changes)

Parse client_state.xml:

Host benchmarks (under <host_info>):

p_fpops (peak FP ops/sec, i.e., GFLOPS if value is 1e9-based),

p_iops,

p_membw,

p_ncpus, OS, etc.


Coprocessors (under <coproc>): enumerate GPUs (vendor, device ordinal).

Applications & versions (under <app>/<app_version>): capture plan_class, GPU usage flags, avg_ncpus, etc.

Workunits/Results:

Map: result_name → {project, app, app_version, wu_name, rsc_fpops_est, rsc_fpops_bound}.

You’ll use rsc_fpops_est as F_est for throughput math.



Keep two hash maps in memory:

RESULT_META[result_name] = { F_est, app, project, app_version, plan_class }
HOST = { p_fpops, p_iops, ncpus, gpus[] }

Refresh these maps if client_state.xml mtime changes (or on a timer, e.g., every 60s with a checksum to avoid churn).


---

2) Poll live state (1–2 Hz) via GUI RPC

Authenticate using the token in gui_rpc_auth.cfg. On each tick:

get_state (or get_tasks on newer clients):

For each active task (result):

result_name

fraction_done (∈[0,1])

elapsed_time (wall-clock)

current_cpu_time (CPU time; useful for throttle/CPU_ratio)

scheduler_state / active_task_state

Slot id (if you want to correlate with slots/)

(If exposed) CPU/GPU resource usage hints



get_host_info (every ~30s): in case p-states/benchmarks updated.


Maintain a short history window per running result:

HIST[result_name] = { last_fraction, last_ts, EMA_rate, etc. }


---

3) Compute instantaneous effective FLOPS from progress deltas

For each active result r:

Definitions:

F_est = RESULT_META[r].F_est  (from client_state.xml)

From RPC at times t-Δt and t: f_prev, f_now, elapsed_prev, elapsed_now

Δf = max(0, f_now - f_prev); ignore if Δf > 0.2 (checkpoint jump clamp; tune cap)

Δt = max(ε, t - (t-Δt))


Raw rate:

R_raw = F_est * (Δf / Δt)         # FLOPs/sec

Smoothed (per-task EMA):

R_task = α * R_raw + (1-α) * R_task_prev   # α ~ 0.2–0.4

Optional throttle awareness:

cpu_ratio = (ΔCPU_time / Δt)
R_task *= max(0.5, cpu_ratio)    # soft-penalize if client is suspended/throttled

Per-host instantaneous:

R_host_now = Σ_active R_task

Also track ETA for each task:

W_rem = (1 - f_now) * F_est
ETA   = W_rem / max(ε, R_task)


---

4) Completion-derived correction & rolling truth

When a result finishes (seen via RPC state change):

Record:

t_elapsed = final_elapsed_time

R_comp = F_est / t_elapsed


Append to an app/plan-class bucket to learn a correction factor:


c_app = median_over_last_K ( R_comp / (p_fpops * eff_cpu_share + projected_gpu_flops) )

or simpler,

adj_F_est = median_over_last_K ( actual_F / F_est )

Use adj_F_est (capped to [0.5, 2.0]) to scale future F_est for that app/version so progress-based rates get more accurate over time.

Keep also a rolling host rate:

R_host_5m = EMA_5m of Σ_completed R_comp over a sliding 5–15m window

This is your “validated recent” number.


---

5) Capacity & utilization (denominator that feels legit)

Compute a conservative peak capacity from host benchmarks:

CPU capacity (current):

CPU_peak = HOST.p_fpops * min(1.0, active_cpu_threads / HOST.ncpus)

GPU capacity:

If your app has known projected_flops per GPU (plan class sometimes encodes this), sum those for GPUs currently running tasks.

Else, omit GPU from capacity (or maintain a learned GPU_eff_fp32 per device from step 4).


Total:

R_cap = CPU_peak + GPU_peak (if known)
utilization = R_host_now / max(ε, R_cap)   # 0..1+

This gives you an honest utilization that moves with real work and real benchmarks.


---

6) Aggregation & what to display

Per task (for debugging/UX):

result_name, app, f_now, R_task_instant (GFLOPS), ETA, elapsed, host share (CPU/GPU), adj_F_est_used?


Per host:

R_host_now (GFLOPS) — instant (progress-based)

R_host_1m (GFLOPS) — EMA(instant)

R_host_15m (GFLOPS) — EMA or average of completions (stable baseline)

R_cap (GFLOPS) and utilization %

efficiency = R_host_15m / R_cap


Per fleet:

Sum the above by project / pool.


> These are all real FLOPs-derived numbers; no synthetic “hash” fiction—yet they respond to OC/power/rig changes because fraction progresses faster and completions come sooner.




---

7) JSON records to emit (every tick)

Per-host tick (1–2 Hz)

{
  "ts": 1694500000,
  "host": "rig-07",
  "project": "silica-boinc",
  "fpops_peak_cpu": 3.2e12,
  "gpus": [{"id":"0000:01:00.0","app":"silica_gpu","active":true}],
  "R_now_flops": 2.85e12,
  "R_ema_60s_flops": 2.73e12,
  "R_cap_flops": 3.80e12,
  "utilization": 0.75,
  "tasks": [
    {"result":"wu_abc_123","app":"silica_gpu","f":0.42,"R_task":1.92e12,"ETA_s":318},
    {"result":"wu_def_009","app":"silica_cpu","f":0.31,"R_task":0.88e12,"ETA_s":640}
  ]
}

On completion

{
  "event":"task_completed",
  "ts":1694500123,
  "host":"rig-07",
  "result":"wu_abc_123",
  "F_est":3.6e14,
  "elapsed_s":1850,
  "R_comp_flops":1.95e11,
  "app":"silica_gpu",
  "plan_class":"cuda125",
  "adj_factor_update":0.97
}


---

8) Minimal control loop (pseudocode)

load_meta_from_client_state()
rpc = connect_gui_rpc(auth_token)
while True:
    state = rpc.get_state()
    now = monotonic()
    R_host_now = 0
    for task in state.active_tasks:
        meta = RESULT_META[task.result_name]
        F_est = meta.F_est * meta.adj_factor  # learned correction
        hist = HIST.get(task.result_name)
        if hist:
            df = clamp(task.fraction_done - hist.f, 0, 0.2)
            dt = max(1e-3, now - hist.ts)
            R_raw = F_est * (df/dt)
            R_task = ema(hist.R_task, R_raw, alpha=0.3)
        else:
            R_task = 0
        HIST[task.result_name] = { "f": task.fraction_done, "ts": now, "R_task": R_task }
        R_host_now += R_task

    R_cap = compute_capacity(HOST, state)  # from p_fpops, gpus running this app, etc.
    emit_tick_json(R_host_now, R_cap, utilization=R_host_now/max(eps,R_cap), tasks=...)

    for finished in detect_newly_finished(state):
        rec = finalize_completion(finished)
        update_adj_factor(rec)  # per app/version bucket
        emit_completion_json(rec)

    sleep(0.5)


---

9) Sanity & edge cases

Nonlinear progress: ignore the first few seconds (until f>0.02) and clamp Δf. The completion-based adjustment will correct systemic bias.

Missing rsc_fpops_est: very rare for serious projects; if missing, use the app bucket’s median F_est from recent jobs as a temporary proxy.

Multi-project rigs: compute per-project R_now by summing only tasks from that project; still use a single host R_cap.

Benchmarks stale: allow an admin command to trigger boinccmd --run_benchmarks and refresh p_fpops.

GPU lanes: keep CPU and each GPU lane separate internally, then sum. (This helps diagnosability without changing the final totals.)



---

TL;DR

Read client_state.xml for F_est (rsc_fpops_est) and host benchmarks; poll GUI RPC for fraction_done and times.

Instant rate = F_est * Δf / Δt (EMA-smoothed).

Validate & learn with completion rates to correct F_est per app/version.

Capacity from p_fpops (+ GPU projected FLOPs if available) → utilization.

Output per-task, per-host, and fleet totals in FLOPs/sec—authentic, responsive, and miner-pleasing without being cheaty.



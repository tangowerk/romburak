import { createResource, createSignal, Show } from "solid-js";
import Panel from "./Panel";
import PhaseProgress from "./PhaseProgress";
import Select from "./Select";
import { raDisplay, raInstall, raSetDisplay, raStatus } from "../ipc";

/** The RetroArch Romburak downloads and runs itself: install, version, update to newest stable. */
export default function ManagedRetroArchPanel() {
  const [check, setCheck] = createSignal(false);
  // Wrapped: a falsy source (`false`) would keep the resource from ever loading.
  const [status, { refetch }] = createResource(() => ({ check: check() }), (s) => raStatus(s.check));
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal("");

  const install = async (version: string | null) => {
    setBusy(true);
    setMsg("");
    try {
      const v = await raInstall(version);
      setMsg(`RetroArch ${v} installed`);
      void refetch();
    } catch (e) {
      setMsg(String(e));
    } finally {
      setBusy(false);
    }
  };
  const [display, { mutate: setDisplay }] = createResource(raDisplay);
  const displayOptions = [
    { value: "", label: "RetroArch decides", hint: "its own menu setting" },
    { value: "fullscreen", label: "Fullscreen" },
    ...[1, 2, 3, 4, 5, 6].map((n) => ({ value: `window:${n}`, label: `Window ×${n}`, hint: `${320 * n}×${240 * n} for 320×240` })),
  ];
  const changeDisplay = async (v: string) => {
    const prev = display() ?? null;
    setDisplay(v || null);
    try {
      await raSetDisplay(v || null);
    } catch (e) {
      setDisplay(prev);
      setMsg(String(e));
    }
  };
  /** Opt-in nightly channel: installed as `nightly-YYYY-MM-DD`. */
  const onNightly = () => !!status()?.installed?.startsWith("nightly");
  const newer = () => {
    const s = status();
    // Without an installation there is nothing to update; the install button covers that case.
    return s?.installed && !onNightly() && s.latest && s.latest !== s.installed ? s.latest : null;
  };

  return (
    <Panel title="Managed RetroArch">
      <Show when={status()} fallback={<p class="dim">{status.error ? String(status.error) : "loading…"}</p>}>
        {(s) => (
          <Show when={s().supported} fallback={<p class="dim">No RetroArch build for this platform.</p>}>
            <div class="kpi">{s().installed ?? "not installed"}</div>
            <p class="dim small">
              <Show
                when={s().nightly}
                fallback={
                  <Show
                    when={onNightly()}
                    fallback={<>Romburak downloads the official stable build (pinned: {s().pinned}, ~200 MB)</>}
                  >
                    Romburak runs the nightly channel (newest development build, untested, possibly broken on any
                    given day; go back to stable {s().pinned} if something misbehaves)
                  </Show>
                }
              >
                There is no stable build for this platform: Romburak downloads the newest nightly plus the menu assets
                (~80 MB). It needs SDL2 and the usual desktop libraries from the system
              </Show>{" "}
              and keeps cores, saves and states in its own folder. Your ROM folders stay untouched.
            </p>
            <p class="dim mono small">{s().folder}</p>
            <div class="row">
              <Show when={!s().installed}>
                <button class="btn" disabled={busy()} onClick={() => void install(null)}>
                  Install RetroArch {s().nightly ? "nightly" : s().pinned}
                </button>
              </Show>
              <Show when={newer()}>
                {(v) => (
                  <button class="btn" disabled={busy()} onClick={() => void install(v())}>
                    Update to {v()}
                  </button>
                )}
              </Show>
              <Show when={!s().nightly && onNightly()}>
                <button class="btn" disabled={busy()} onClick={() => void install("nightly")}>
                  Update nightly
                </button>
                <button class="btn ghost" disabled={busy()} onClick={() => void install(null)}>
                  Back to stable {s().pinned}
                </button>
              </Show>
              <Show when={!s().nightly && s().installed && !onNightly()}>
                <button
                  class="btn ghost"
                  disabled={busy() || status.loading}
                  onClick={() => (check() ? void refetch() : setCheck(true))}
                >
                  Check for updates
                </button>
                <button
                  class="btn ghost"
                  disabled={busy()}
                  title="Newest development build: more recent fixes, but untested. Cores are always the newest anyway."
                  onClick={() => void install("nightly")}
                >
                  Try nightly
                </button>
              </Show>
            </div>
            <div class="row">
              <span class="dim small">Games start in</span>
              <Select value={display() ?? ""} options={displayOptions} onChange={(v) => void changeDisplay(v)} />
            </div>
            <Show when={check() && !status.loading && !newer() && s().installed}>
              <p class="dim small">
                {s().latest ? `Up to date (newest stable: ${s().latest})` : "Buildbot not reachable"}
              </p>
            </Show>
          </Show>
        )}
      </Show>
      <Show when={busy()} fallback={<Show when={msg()}><p class="dim small">{msg()}</p></Show>}>
        <PhaseProgress
          event="ra://progress"
          labels={{ download: "downloading (MB)", verify: "verifying", unpack: "unpacking (MB)" }}
        />
      </Show>
    </Panel>
  );
}

import { For, Show, type Resource } from "solid-js";
import Panel from "./Panel";
import type { CheevosLive, RaCheck } from "../ipc";

/** RA dates are UTC text ("2026-11-23 20:15:00"); shown in local time. */
const raDate = (s: string) => {
  const d = new Date(s.replace(" ", "T") + "Z");
  return Number.isNaN(d.getTime()) ? s : d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
};

function Light<T>(props: { label: string; check: RaCheck<T> | null; missing: string }) {
  const state = () => (!props.check ? "off" : props.check.ok ? "ok" : "bad");
  return (
    <li class={`light ${state()}`}>
      <span class="light-dot" aria-hidden="true" />
      <span>
        <strong>{props.label}</strong>{" "}
        <span class="dim small">
          {state() === "ok" ? "works" : state() === "off" ? props.missing : props.check?.error}
        </span>
      </span>
    </li>
  );
}

/** Is RetroAchievements live? Profile from the site, a light per credential, the newest unlocks. */
export default function CheevosStatusPanel(props: { live: Resource<CheevosLive>; onCheck: () => void }) {
  const summary = () => props.live()?.key?.ok ?? null;
  const login = () => props.live()?.login?.ok ?? null;
  const avatar = () => summary()?.avatar || login()?.avatar_url || "";
  const lastOk = () => {
    const t = props.live()?.last_ok;
    return t ? new Date(t * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" }) : "never";
  };
  return (
    <Panel title="Status" class="wide">
      <div class="ra-head">
        <Show when={avatar()} fallback={<div class="ra-avatar empty" />}>
          <img class="ra-avatar" src={avatar()} alt="" />
        </Show>
        <div class="ra-who">
          <Show when={login() ?? summary()} fallback={<strong>Not connected</strong>}>
            {(p) => <strong class="ra-name">{p().user}</strong>}
          </Show>
          <Show when={summary() ?? login()}>
            <span class="small">
              <span class="accent">{(summary()?.points ?? login()?.score ?? 0).toLocaleString()}</span> hardcore ·{" "}
              {(summary()?.softcore_points ?? login()?.softcore_score ?? 0).toLocaleString()} softcore points
              <Show when={summary()?.rank}>
                {(r) => <> · rank {r().toLocaleString()} of {summary()?.total_ranked?.toLocaleString()}</>}
              </Show>
            </span>
          </Show>
          <Show when={summary()?.rich_presence}>
            <span class="dim small">Last seen: {summary()?.rich_presence}</span>
          </Show>
        </div>
        <button class="btn ghost small" disabled={props.live.loading} onClick={() => props.onCheck()}>
          {props.live.loading ? "Checking…" : "Check again"}
        </button>
      </div>
      <ul class="lights">
        <Light label="Login" check={props.live()?.login ?? null} missing="not logged in" />
        <Light
          label="Web API key"
          check={props.live()?.key ?? null}
          missing={props.live()?.login ? "no key stored" : "log in first"}
        />
        <li class="dim small">last successful check: {lastOk()}</li>
      </ul>
      <Show when={props.live.error}>
        <p class="err small">{String(props.live.error)}</p>
      </Show>
      <Show when={summary()?.recent.length}>
        <h4 class="step">Latest unlocks</h4>
        <ul class="ra-unlocks">
          <For each={summary()!.recent}>
            {(u) => (
              <li>
                <img src={u.badge} alt="" loading="lazy" />
                <span class="ra-unlock-text">
                  <strong>{u.title}</strong>
                  <span class="dim small">
                    {u.game_title} · {u.points} pts{u.hardcore ? " · hardcore" : ""} · {raDate(u.date)}
                  </span>
                </span>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </Panel>
  );
}

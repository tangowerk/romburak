import { createResource, createSignal, For, Show } from "solid-js";
import Panel from "./Panel";
import { cheevosOptions, cheevosSetHardcore, cheevosSetOption, cheevosStatus } from "../ipc";

const LABELS: Record<string, [string, string]> = {
  unlock_popup: ["Unlock pop-ups", "notification when you unlock an achievement or master a game"],
  unlock_sound: ["Unlock sound", "plays a sound on unlock"],
  unlock_screenshot: ["Screenshot on unlock", "saves a screenshot into RetroArch's screenshot folder"],
  leaderboards: ["Leaderboard pop-ups", "start, submit and live trackers of leaderboard attempts"],
  challenge_indicators: ["Challenge indicators", "small badge while an achievement can be won right now"],
  menu_badges: ["Badges in RetroArch's menu", "downloads achievement badge pictures"],
  encore: ["Encore mode", "earn achievements again that you already unlocked (for fun, not counted)"],
};

/** RetroArch's in-game achievement settings, written into the managed RetroArch's config. */
export default function CheevosOptionsPanel() {
  const [status, { refetch, mutate }] = createResource(cheevosStatus);
  const [options, { refetch: refetchOptions }] = createResource(cheevosOptions);
  const [msg, setMsg] = createSignal("");
  const hardcore = async (on: boolean) => {
    const s = status();
    if (s) mutate({ ...s, hardcore: on });
    try {
      await cheevosSetHardcore(on);
    } catch (e) {
      setMsg(String(e));
      void refetch();
    }
  };
  const set = async (id: string, on: boolean) => {
    try {
      await cheevosSetOption(id, on);
    } catch (e) {
      setMsg(String(e));
    }
    void refetchOptions();
  };
  return (
    <Panel title="While playing">
      <p class="dim small">Applies to games started from Romburak, from the next start.</p>
      <label class="check">
        <input type="checkbox" checked={status()?.hardcore ?? false} onChange={(e) => void hardcore(e.currentTarget.checked)} />
        <span>
          Hardcore mode <span class="dim small">– no savestates, rewind or cheats; unlocks count as hardcore on the site</span>
        </span>
      </label>
      <For each={options() ?? []}>
        {(o) => (
          <label class="check">
            <input type="checkbox" checked={o.on} onChange={(e) => void set(o.id, e.currentTarget.checked)} />
            <span>
              {LABELS[o.id]?.[0] ?? o.id} <span class="dim small">– {LABELS[o.id]?.[1] ?? ""}</span>
            </span>
          </label>
        )}
      </For>
      <Show when={msg()}>
        <p class="err small">{msg()}</p>
      </Show>
    </Panel>
  );
}

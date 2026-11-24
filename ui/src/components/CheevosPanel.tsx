import { createResource, createSignal, Show } from "solid-js";
import Panel from "./Panel";
import PhaseProgress from "./PhaseProgress";
import { cheevosStatus, cheevosSync, type RaCounts } from "../ipc";
import { refreshLibrary } from "../state/libraryStore";
import { cancelPlayers, playersError, playersProgress, refreshPlayers } from "../state/popularity";
import Segments from "./Segments";

/** Library match: RA game lists + hashing the library, so it shows which games have achievements. */
export default function CheevosPanel(props: { counts: RaCounts | undefined; onChange: () => void }) {
  const [status, { refetch }] = createResource(cheevosStatus);
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal("");
  const pct = () => {
    const c = props.counts;
    return c && c.files ? Math.round((100 * c.files_with) / c.files) : 0;
  };
  const sync = async () => {
    setBusy(true);
    setMsg("");
    try {
      const n = await cheevosSync();
      setMsg(`${n} library games have achievements`);
      void refetch();
      props.onChange();
      void refreshLibrary().then(refreshPlayers);
    } catch (e) {
      setMsg(String(e));
    } finally {
      setBusy(false);
    }
  };
  const synced = () => {
    const t = status()?.synced;
    return t ? new Date(t * 1000).toLocaleString() : "never";
  };

  return (
    <Panel title="Library match">
      <p class="dim small">
        Downloads RetroAchievements' game lists and hashes your library the way RetroArch does, so the library
        shows which games have achievements.
      </p>
      <Show when={props.counts?.games}>
        <dl class="facts">
          <dt>At RetroAchievements</dt>
          <dd>{props.counts!.games.toLocaleString()} games on {props.counts!.consoles} consoles</dd>
          <dt>Your library</dt>
          <dd>
            {props.counts!.files_with.toLocaleString()} of {props.counts!.files.toLocaleString()} files have
            achievements ({pct()} %)
          </dd>
        </dl>
      </Show>
      <div class="row">
        <button class="btn" disabled={busy() || !status()?.has_key} onClick={() => void sync()}>
          Sync achievements
        </button>
        <span class="dim small">last sync: {synced()}</span>
        <Show when={!status()?.has_key}>
          <span class="dim small">· needs the Web API key</span>
        </Show>
      </div>
      <Show when={busy()} fallback={<Show when={msg()}><p class="dim small">{msg()}</p></Show>}>
        <PhaseProgress
          event="cheevos://progress"
          labels={{ sync: "downloading game lists", hash: "hashing library" }}
        />
      </Show>
      <Show when={playersProgress()?.total}>
        <Segments value={playersProgress()?.done ?? 0} max={playersProgress()?.total ?? 0} />
        <p class="dim small">
          fetching player counts (popularity) {playersProgress()?.done}/{playersProgress()?.total} ·{" "}
          <button class="btn ghost sm" onClick={cancelPlayers}>Stop</button>
        </p>
      </Show>
      <Show when={playersError()}>
        <p class="dim small">Popularity: {playersError()}</p>
      </Show>
    </Panel>
  );
}

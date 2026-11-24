// Settings → RetroArch → Picture: global shader preset (searchable) and aspect ratio.
import { createMemo, createResource, createSignal, For, onCleanup, Show } from "solid-js";
import Pager, { createPaged } from "./Pager";
import Panel from "./Panel";
import PhaseProgress from "./PhaseProgress";
import Select from "./Select";
import { onPlayEnded, raRunning, raSetAspect, raSetShader, raShaders, raVideo } from "../ipc";
import { SHADER_CATEGORIES, shaderCategory } from "../shaderCategories";

const ASPECTS = [
  { value: "", label: "RetroArch decides", hint: "its own menu setting" },
  { value: "core", label: "Core provided" },
  { value: "4:3", label: "4:3" },
  { value: "16:9", label: "16:9" },
  { value: "square", label: "Square pixels", hint: "1:1 PAR" },
  { value: "full", label: "Stretch to window" },
];

export default function PicturePanel() {
  const [video, { refetch }] = createResource(raVideo);
  const [load, setLoad] = createSignal(false);
  // Bounded wait: a call that never answers shows an error with a retry instead of loading forever.
  const [presets, { refetch: refetchPresets }] = createResource(load, () =>
    Promise.race([
      raShaders(),
      new Promise<never>((_, fail) => setTimeout(() => fail(new Error("no answer from the backend after 5 min")), 300_000)),
    ]),
  );
  const [running, { refetch: refetchRunning }] = createResource(raRunning);
  const poll = setInterval(() => void refetchRunning(), 3000);
  const unlisten = onPlayEnded(() => void refetchRunning());
  onCleanup(() => (clearInterval(poll), void unlisten.then((f) => f())));
  const [query, setQuery] = createSignal("");
  const [category, setCategory] = createSignal("");
  const categoryOptions = createMemo(() => {
    const counts = new Map<string, number>();
    for (const p of presets() ?? []) counts.set(shaderCategory(p.path), (counts.get(shaderCategory(p.path)) ?? 0) + 1);
    return [
      { value: "", label: "All categories", hint: `${presets()?.length ?? 0}` },
      ...[...counts].sort(([a], [b]) => a.localeCompare(b)).map(([c, n]) => ({ value: c, label: c, hint: `${n}` })),
    ];
  });
  const [msg, setMsg] = createSignal("");
  const matches = createMemo(() => {
    const words = query().toLowerCase().split(/\s+/).filter(Boolean);
    return (presets() ?? []).filter(
      (p) =>
        (!category() || shaderCategory(p.path) === category()) && words.every((w) => p.path.toLowerCase().includes(w)),
    );
  });
  const paged = createPaged(matches, () => 12);
  const run = async (f: () => Promise<unknown>, ok: string) => {
    try {
      const live = await f();
      setMsg(live === true ? `${ok} · applied to the running game` : ok);
    } catch (e) {
      setMsg(String(e));
    }
    void refetch();
  };
  const shader = () => video()?.shader ?? null;
  const shaderLabel = () =>
    shader() === null ? "RetroArch decides" : shader() === "off" ? "Off" : shader()!.replace(/\.slangp$/, "");
  return (
    <Panel title="Picture" class="wide">
      <p class="dim small">Applies to every game started from Romburak. Aspect ratio: next start.</p>
      <Show when={running()}>
        {(p) => <p class="small live-note">● Running: <span class="mono">{p()}</span> – shader changes apply live.</p>}
      </Show>
      <div class="row wrap">
        <label class="dim small">Aspect ratio</label>
        <Select
          value={video()?.aspect ?? ""}
          options={ASPECTS}
          onChange={(v) => void run(() => raSetAspect(v || null), "Aspect ratio saved")}
        />
      </div>
      <div class="row wrap">
        <label class="dim small">Shader</label>
        <strong class="mono small">{shaderLabel()}</strong>
        <button class="btn ghost small" disabled={shader() === "off"} onClick={() => void run(() => raSetShader("off"), "Shader off")}>
          Off
        </button>
        <button class="btn ghost small" disabled={shader() === null} onClick={() => void run(() => raSetShader(null), "Shader left to RetroArch")}>
          RetroArch decides
        </button>
        <Show when={!load() || presets.error}>
          <button class="btn small" onClick={() => (load() ? void refetchPresets() : setLoad(true))}>
            {presets.error ? "Retry" : "Choose preset…"}
          </button>
        </Show>
      </div>
      <Show when={load()}>
        <Show
          when={!presets.loading}
          fallback={
            <>
              <p class="dim small">Loading shaders (only without bundled shaders: downloading the slang package, ~55 MB)…</p>
              <PhaseProgress event="ra://package" labels={{ download: "downloading (MB)", unpack: "unpacking" }} />
            </>
          }
        >
          <Show when={!presets.error} fallback={<p class="err small">{String(presets.error)}</p>}>
            <div class="row">
              <Select value={category()} options={categoryOptions()} onChange={setCategory} />
              <input
                class="field preset-search"
                type="search"
                placeholder={`Search ${matches().length} presets, e.g. "crt royale"`}
                value={query()}
                onInput={(e) => setQuery(e.currentTarget.value)}
              />
            </div>
            <Show when={category()}>
              <p class="dim small">{SHADER_CATEGORIES[category()] ?? ""}</p>
            </Show>
            <ul class="preset-list">
              <For each={paged.items()}>
                {(p) => (
                  <li>
                    <button
                      class="preset mono small"
                      classList={{ sel: shader() === p.path }}
                      title={SHADER_CATEGORIES[shaderCategory(p.path)] ?? ""}
                      onClick={() => void run(() => raSetShader(p.path), `Shader: ${p.path}`)}
                    >
                      <span>{p.path.replace(/\.slangp$/, "")}</span>
                      <Show when={p.passes}>
                        {(n) => (
                          <span class="preset-passes dim" title="Shader passes: more passes = more GPU load">
                            {n()} {n() === 1 ? "pass" : "passes"}
                          </span>
                        )}
                      </Show>
                    </button>
                  </li>
                )}
              </For>
            </ul>
            <Pager paged={paged} />
          </Show>
        </Show>
      </Show>
      <Show when={msg()}><p class="dim small">{msg()}</p></Show>
    </Panel>
  );
}

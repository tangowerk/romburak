import { createResource, createSignal, Show } from "solid-js";
import Panel from "./Panel";
import { cheevosLogin, cheevosLogout, cheevosOpenSettings, cheevosSetKey, cheevosStatus } from "../ipc";
import { refreshProgress } from "../state/libraryStore";

/** RetroAchievements access, both in one place: the login (RetroArch unlocks while playing)
 *  and the Web API key (everything Romburak shows about achievements). */
export default function CheevosAccountPanel(props: { onChange: () => void }) {
  const [status, { refetch }] = createResource(cheevosStatus);
  const [user, setUser] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [key, setKey] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal("");
  const changed = () => (void refetch(), props.onChange(), void refreshProgress());

  const login = async () => {
    setBusy(true);
    setMsg("");
    try {
      await cheevosLogin(user().trim(), password());
      setUser("");
      changed();
    } catch (e) {
      setMsg(String(e));
    } finally {
      setPassword("");
      setBusy(false);
    }
  };
  const logout = async () => {
    await cheevosLogout();
    changed();
  };
  const saveKey = async () => {
    setBusy(true);
    setMsg("");
    try {
      await cheevosSetKey(key());
      setKey("");
      setMsg("Key works and is saved.");
      changed();
    } catch (e) {
      setMsg(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Account">
      <h4 class="step">1 · Login <span class="dim small">for RetroArch: unlocks while you play</span></h4>
      <Show
        when={status()?.user}
        fallback={
          <>
            <p class="dim small">Your password is sent once to retroachievements.org; only the login token is kept.</p>
            <form
              class="row"
              onSubmit={(e) => {
                e.preventDefault();
                void login();
              }}
            >
              <input
                class="field"
                placeholder="User name"
                autocomplete="username"
                value={user()}
                onInput={(e) => setUser(e.currentTarget.value)}
              />
              <input
                class="field"
                type="password"
                placeholder="Password"
                autocomplete="current-password"
                value={password()}
                onInput={(e) => setPassword(e.currentTarget.value)}
              />
              <button class="btn" type="submit" disabled={busy() || !user().trim() || !password()}>
                Log in
              </button>
            </form>
          </>
        }
      >
        <div class="row">
          <span>
            Logged in as <strong>{status()?.user}</strong>
          </span>
          <button class="btn ghost" onClick={() => void logout()}>
            Log out
          </button>
        </div>
      </Show>
      <h4 class="step">2 · Web API key <span class="dim small">for Romburak: which games have achievements, your progress</span></h4>
      <p class="dim small">
        Copy it from{" "}
        <button class="link" onClick={() => void cheevosOpenSettings()}>
          retroachievements.org → Settings → Authentication
        </button>
        {status()?.has_key ? " · a key is stored" : ""}.
      </p>
      <form
        class="row"
        onSubmit={(e) => {
          e.preventDefault();
          void saveKey();
        }}
      >
        <input
          class="field"
          type="password"
          placeholder={status()?.has_key ? "Replace Web API key" : "Web API key"}
          value={key()}
          onInput={(e) => setKey(e.currentTarget.value)}
        />
        <button class="btn" classList={{ ghost: !!status()?.has_key }} type="submit" disabled={busy() || !key().trim()}>
          {busy() ? "Checking…" : "Save key"}
        </button>
      </form>
      <Show when={msg()}>
        <p class="small" classList={{ err: !msg().startsWith("Key works") }}>{msg()}</p>
      </Show>
    </Panel>
  );
}

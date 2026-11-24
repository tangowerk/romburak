import { createResource } from "solid-js";
import CheevosAccountPanel from "../components/CheevosAccountPanel";
import CheevosOptionsPanel from "../components/CheevosOptionsPanel";
import CheevosPanel from "../components/CheevosPanel";
import CheevosStatusPanel from "../components/CheevosStatusPanel";
import { cheevosLive } from "../ipc";

/** Settings → Achievements: live status, account (login + Web API key), library match, in-game options. */
export default function AchievementsSettings() {
  const [live, { refetch }] = createResource(cheevosLive);
  const check = () => void refetch();
  return (
    <div class="settings">
      <CheevosStatusPanel live={live} onCheck={check} />
      <CheevosAccountPanel onChange={check} />
      <CheevosPanel counts={live()?.counts} onChange={check} />
      <CheevosOptionsPanel />
    </div>
  );
}

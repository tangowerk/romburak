import CorePicker from "../components/CorePicker";
import ManagedRetroArchPanel from "../components/ManagedRetroArchPanel";
import Panel from "../components/Panel";
import PicturePanel from "../components/PicturePanel";

/** Everything RetroArch: the managed RetroArch, picture and the core per system (achievements: Settings). */
export default function RetroArch() {
  return (
    <div class="grid">
      <div class="wide">
        <ManagedRetroArchPanel />
      </div>
      <PicturePanel />
      <Panel title="Cores per system" class="wide">
        <CorePicker />
      </Panel>
    </div>
  );
}

import { createEffect, createSignal, Show } from "solid-js";
import styles from "./home.module.css";
import { err, conflict_labels } from "./signals";
import { UploadTab } from "./UploadTab";
import { DownloadTab } from "./DownloadTab";
import { ExportTab } from "./ExportTab";
import { ConflictTab } from "./ConflictTab";
import { PasswordTab } from "./PasswordTab";

const NAV_TABS = [
  "All Items",
  "Export",
  "Upload",
  "Download",
  "Conflicts",
] as const;
type NavTabs = (typeof NAV_TABS)[number];

const [curr_tab, set_curr_tab] = createSignal<NavTabs>("All Items");

export const Home = () => {
  createEffect(() => {
    if (err()) {
      console.error("ERR: ", err());
    }
  });
  const tabs = () => {
    if (conflict_labels().length > 0) {
      return NAV_TABS;
    }

    return NAV_TABS.slice(0, NAV_TABS.length - 1);
  };

  return (
    <div class={styles.container}>
      <div class={styles.sidebar}>
        <ul class={styles.navigation}>
          {tabs().map((tab_name) => (
            <Tab tab_name={tab_name} />
          ))}
        </ul>
      </div>
      <div class={styles.content_container}>
        <Show when={curr_tab() === "All Items"}>
          <PasswordTab on_conflict_click={() => set_curr_tab("Conflicts")} />
        </Show>
        <Show when={curr_tab() === "Export"}>
          <ExportTab />
        </Show>
        <Show when={curr_tab() === "Upload"}>
          <UploadTab />
        </Show>
        <Show when={curr_tab() === "Download"}>
          <DownloadTab />
        </Show>
        <Show when={curr_tab() === "Conflicts"}>
          <ConflictTab />
        </Show>
      </div>
    </div>
  );
};

function Tab(props: { tab_name: NavTabs }) {
  return (
    <li
      style={
        curr_tab() === props.tab_name ? { "text-decoration": "underline" } : {}
      }
      onClick={() => set_curr_tab(props.tab_name)}
    >
      {props.tab_name}
    </li>
  );
}

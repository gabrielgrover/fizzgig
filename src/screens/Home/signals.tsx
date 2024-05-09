import { createSignal, createResource } from "solid-js";
import { invoke } from "@tauri-apps/api/tauri";
import * as TE from "fp-ts/TaskEither";

export const [err, set_err] = createSignal("");

export const [conflict_labels, set_conflict_labels] = createSignal<string[]>(
  []
);

export function createLazyPwLoader() {
  return {
    data: pw_meta_data,
    load: () => {
      if (!should_load_pw_meta_data()) {
        set_should_load_pw_meta_data(true);
      }

      refetch_pw_meta_data();
    },
  };
}

export const [sync_in_progress, set_sync_in_progress] = createSignal(false);

const [should_load_pw_meta_data, set_should_load_pw_meta_data] =
  createSignal(false);

const [pw_meta_data, { refetch: refetch_pw_meta_data }] = createResource(
  should_load_pw_meta_data,
  get_pw_meta_data
);

function get_pw_meta_data(should_fetch: boolean) {
  return TE.tryCatch(
    async () => {
      console.log("FETCHING PW DATA");
      if (!should_fetch) {
        return [];
      }

      const r = await invoke<{ label: string; has_conflict: true }[]>("list");

      console.log("PW DATA: ", r);

      return r;
    },
    (err) => {
      if (typeof err === "string") {
        return err;
      }

      return `An unknown error occurred: ${JSON.stringify(err, null, 2)}`;
    }
  )();
}

import { createSignal } from "solid-js";

const [master_pw, set_master_pw] = createSignal<string>();

export const master_pass_signal = {
  master_pw,
  set_master_pw,
};

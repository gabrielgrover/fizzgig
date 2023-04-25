import { Component, createSignal } from "solid-js";

type Props = {
  on_submit: (data: { label: string; pw: string }) => void;
};

export const AddPassword: Component<Props> = (props) => {
  const [pw, set_pw] = createSignal("");
  const [pw_label, set_pw_label] = createSignal("");

  return (
    <div class="row">
      <div>
        <input
          onChange={(e) => set_pw_label(e.currentTarget.value)}
          placeholder="Enter password label..."
        />
        <input
          onChange={(e) => {
            set_pw(e.currentTarget.value);
          }}
          placeholder="Enter password..."
        />
        <button
          type="button"
          onClick={() => props.on_submit({ label: pw_label(), pw: pw() })}
        >
          Submit
        </button>
      </div>
    </div>
  );
};

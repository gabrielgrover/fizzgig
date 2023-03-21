import { createSignal } from "solid-js";

type Props = {
	 
};

export const AddPassword = () => {
	const [pw, set_pw] = createSignal("");
	const [pw_label, set_pw_label] = createSignal("");

	return (
      <div class="row">
        <div>
					<input onChange={e => {set_pw_label(e.currentTarget.value)}} placeholder="Enter password..." />
          <input
            onChange={(e) => set_pw(e.currentTarget.value)}
            placeholder="Enter password label..."
          />
					<button type="button" onClick={() => null}>
            Greet
          </button>
        </div>
      </div>
	);
};

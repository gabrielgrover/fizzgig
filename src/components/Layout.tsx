import { ParentComponent } from "solid-js";
import { Link } from "@solidjs/router";

export const Layout: ParentComponent = (props) => {
  return (
    <div>
      <header>
        <nav>
          <ul>
            <li>
              <Link href="/">Home</Link>
            </li>
            <li>
              <Link href="/passwords">Passwords</Link>
            </li>
          </ul>
        </nav>
      </header>
      {props.children}
      <footer>
        <p>&copy; 2023 My Company</p>
      </footer>
    </div>
  );
};

import { Router, Route, Routes } from "@solidjs/router";
import "./App.css";

import { Login, Passwords, Home } from "./screens";

function App() {
  return (
    <Router>
      <Routes>
        <Route path="/" component={Login} />
        <Route path="/home" component={Home} />
        <Route path="/passwords" component={Passwords} />
      </Routes>
    </Router>
  );
}

export default App;

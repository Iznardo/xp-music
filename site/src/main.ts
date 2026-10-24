import { mount } from "svelte";
import "../../src/styles/tokens.css";
import "../../src/styles/base.css";
import "./site.css";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app")! });

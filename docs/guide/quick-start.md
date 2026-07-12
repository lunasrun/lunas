# Quick start

The fastest way to a running Lunas app is the `create-lunas` scaffolder, which
sets up a [Vite](https://vitejs.dev) project wired to `vite-plugin-lunas`.

## Scaffold a project

```sh
npm create lunas@latest my-app
# or, to be prompted for a project name:
npm create lunas@latest
```

Then:

```sh
cd my-app
npm install
npm run dev
```

You get a minimal Vite app:

- `index.html` + `src/main.mjs` — mounts the app into `#app`.
- `src/App.lunas` — a root component with reactive state, an event handler, and
  `:if` / `:for` in the template.
- `vite.config.mjs` — the Lunas Vite plugin.

## Add Lunas to an existing Vite project

If you already have a Vite project, install the plugin and the runtime:

```sh
npm install -D vite-plugin-lunas
npm install lunas
```

`vite` is a peer dependency (`>=5`). Register the plugin:

```js
// vite.config.mjs
import { defineConfig } from "vite";
import lunas from "vite-plugin-lunas";

export default defineConfig({
  plugins: [lunas()],
});
```

The plugin compiles `.lunas` and `.lun` files into ES modules. Compiler
**errors** abort the build with a file/line/column code frame; **warnings** and
**hints** are surfaced but don't stop the build.

## Your first component

A `.lunas` file has up to three blocks, each introduced by a label at column 0:

```lunas
html:
    <main class="app">
        <h1>Lunas Counter</h1>
        <p>Count: ${count}</p>
        <button @click="increment()">+1</button>
        <button @click="reset()">Reset</button>
        <ul>
            <li :for="n of history">Was ${n}</li>
        </ul>
    </main>

style:
    .app { font-family: sans-serif; max-width: 40rem; margin: 2rem auto; }
    button { margin-right: 0.5rem; }

script:
    let count = 0
    let history = []
    function increment() {
        history = [...history, count]
        count = count + 1
    }
    function reset() {
        count = 0
        history = []
    }
```

- `html:` — the template. `${count}` interpolates a value; `@click="increment()"`
  wires an event; `:for` renders a list. See [Template syntax](./template-syntax.md).
- `style:` — plain CSS (optional).
- `script:` — plain JavaScript (or TypeScript). Mutating a top-level `let` makes
  it reactive; that's all the reactivity setup there is. See
  [Reactivity fundamentals](./reactivity-fundamentals.md).

## Mounting

A compiled component's **default export is a factory function**. Call it (with
props, if any) to build a *detached* root, then `attach` it to a live host
element:

```js
// src/main.mjs
import { attach } from "lunas";
import App from "./App.lunas";

// App() builds a detached root; attach() inserts it and fires onMount.
attach(App(), document.getElementById("app"));
```

`attach(root, host)` appends the root to the host and fires the whole subtree's
[`onMount`](./lifecycle.md) callbacks. The component is built off-DOM and touches
the live tree exactly once — this is a core part of what makes the initial render
fast.

## What next

- Learn the [template syntax](./template-syntax.md) in full.
- Understand [reactivity](./reactivity-fundamentals.md) and the flush model.
- Explore [components & props](../components/) to split your UI into pieces.

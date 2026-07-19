// madore フロントエンド（vanilla JS）
// - /api/tree でツリー取得・描画
// - /api/content?path= で本文取得・描画 + 後処理（highlight.js / KaTeX / mermaid）
// - /events (SSE) でライブリロード

const state = {
  currentPath: null,
  // 現在表示中ファイルのレスポンス（html / raw の両方を保持し、モード切替で再取得しない）。
  currentData: null,
  treeSignature: null,
  // Raw（生 markdown テキスト）表示モードか。localStorage に永続化する。
  rawMode: loadRawMode(),
};

const el = {
  tree: document.getElementById("tree"),
  rootName: document.getElementById("root-name"),
  doc: document.getElementById("doc"),
  raw: document.getElementById("raw"),
  toggle: document.getElementById("view-toggle"),
  empty: document.getElementById("empty"),
};

function loadRawMode() {
  try {
    return localStorage.getItem("madore:rawMode") === "1";
  } catch (_) {
    return false;
  }
}

// ---- ツリー ----

async function loadTree() {
  const res = await fetch("/api/tree");
  if (!res.ok) return;
  const data = await res.json();

  // ツリー構造に変化が無ければ DOM を作り直さない。
  // （ファイル保存のたびに再構築すると、クリック中の要素が差し替わって
  //   クリックが無効化されたり、開閉状態が失われたりするのを防ぐ）
  const signature = JSON.stringify(data);
  if (signature === state.treeSignature) return;
  state.treeSignature = signature;

  // 再構築で失われる「畳んでいたフォルダ」とスクロール位置を控えて復元する。
  const closedDirs = new Set(
    [...el.tree.querySelectorAll("details")]
      .filter((d) => !d.open)
      .map((d) => d.dataset.path),
  );
  const scrollTop = el.tree.scrollTop;

  el.rootName.textContent = data.root_name;
  el.tree.innerHTML = "";
  el.tree.appendChild(renderNodes(data.nodes, closedDirs));
  el.tree.scrollTop = scrollTop;

  // 初回で未選択なら先頭ファイルを自動で開く（"not selected" を出さない）。
  if (!state.currentPath) {
    const first = firstFile(data.nodes);
    if (first) openFile(first);
  } else {
    highlightActive();
  }
}

function firstFile(nodes) {
  // 同階層のファイルを優先し、無ければサブディレクトリへ降りる。
  for (const node of nodes) {
    if (!node.is_dir) return node.path;
  }
  for (const node of nodes) {
    if (node.is_dir) {
      const found = firstFile(node.children);
      if (found) return found;
    }
  }
  return null;
}

function renderNodes(nodes, closedDirs) {
  const closed = closedDirs || new Set();
  const ul = document.createElement("ul");
  for (const node of nodes) {
    const li = document.createElement("li");
    if (node.is_dir) {
      const details = document.createElement("details");
      // 以前畳まれていたフォルダは畳んだまま、それ以外（新規含む）は開く。
      details.open = !closed.has(node.path);
      details.dataset.path = node.path;
      const summary = document.createElement("summary");
      summary.className = "dir";
      summary.textContent = node.name;
      details.appendChild(summary);
      details.appendChild(renderNodes(node.children, closed));
      li.appendChild(details);
    } else {
      const a = document.createElement("a");
      a.className = "file";
      a.textContent = node.name;
      a.href = "#";
      a.dataset.path = node.path;
      a.addEventListener("click", (e) => {
        e.preventDefault();
        openFile(node.path);
      });
      li.appendChild(a);
    }
    ul.appendChild(li);
  }
  return ul;
}

function highlightActive() {
  for (const a of el.tree.querySelectorAll("a.file")) {
    const active = a.dataset.path === state.currentPath;
    a.classList.toggle("active", active);
    if (active) {
      a.setAttribute("aria-current", "page");
    } else {
      a.removeAttribute("aria-current");
    }
  }
}

// ---- 本文 ----

let openSeq = 0;
async function openFile(path) {
  // 連打・競合対策: 最新のリクエストの結果だけを反映する。
  const seq = ++openSeq;
  const res = await fetch(`/api/content?path=${encodeURIComponent(path)}`);
  if (seq !== openSeq) return;
  if (!res.ok) {
    // 表示中ファイルが削除・リネームで開けなくなったら本文をクリアして未選択に戻す。
    if (state.currentPath === path) clearDoc();
    return;
  }
  const data = await res.json();
  if (seq !== openSeq) return;
  state.currentPath = data.path;
  state.currentData = data;
  document.title = `${data.title} — madore`;

  renderDoc();
  highlightActive();
  el.doc.scrollTop = 0;
  window.scrollTo(0, 0);
}

// 現在の表示モード（レンダリング / Raw）に従って本文を描画する。
// 取得済みの currentData を使うので、モード切替でネットワークアクセスは発生しない。
function renderDoc() {
  const data = state.currentData;
  if (!data) return;

  if (state.rawMode) {
    // 生 markdown をそのままテキストとして表示（textContent なので HTML は解釈されない）。
    el.raw.textContent = data.raw;
    el.raw.hidden = false;
    el.doc.hidden = true;
    el.doc.innerHTML = "";
  } else {
    el.doc.innerHTML = data.html;
    el.doc.hidden = false;
    el.raw.hidden = true;
    el.raw.textContent = "";
    enhance(el.doc);
  }
  el.empty.hidden = true;
  el.toggle.hidden = false;
}

// 表示中ファイルを消して未選択状態に戻す。
function clearDoc() {
  state.currentPath = null;
  state.currentData = null;
  el.doc.innerHTML = "";
  el.doc.hidden = true;
  el.raw.textContent = "";
  el.raw.hidden = true;
  el.empty.hidden = false;
  el.toggle.hidden = true;
  document.title = "madore";
  highlightActive();
}

// ---- 表示モード切替 ----

function applyRawMode() {
  el.toggle.setAttribute("aria-pressed", state.rawMode ? "true" : "false");
  // ボタンは「押すと切り替わる先」を表示する。
  el.toggle.textContent = state.rawMode ? "Rendered" : "Raw";
  el.toggle.title = state.rawMode
    ? "レンダリング表示に切り替え"
    : "Raw（生テキスト）表示に切り替え";
}

function setRawMode(on) {
  state.rawMode = on;
  try {
    localStorage.setItem("madore:rawMode", on ? "1" : "0");
  } catch (_) {
    /* localStorage 不可でもモード自体は動く */
  }
  applyRawMode();
  renderDoc();
}

el.toggle.addEventListener("click", () => setRawMode(!state.rawMode));
applyRawMode();

// レンダリング済み HTML に見た目を付与する後処理。
function enhance(root) {
  renderMermaid(root);
  renderMath(root);
  renderHighlight(root);
}

function renderHighlight(root) {
  if (typeof hljs === "undefined") return;
  root.querySelectorAll("pre code").forEach((block) => {
    // mermaid ブロックは図に置き換えるので対象外。
    if (block.classList.contains("language-mermaid")) return;
    hljs.highlightElement(block);
  });
}

// comrak は `$...$` を <span data-math-style="inline|display"> に変換済み。
// auto-render は使わず、その span を走査して KaTeX 描画する（二重処理回避）。
function renderMath(root) {
  if (typeof katex === "undefined") return;
  root.querySelectorAll("span[data-math-style]").forEach((span) => {
    const display = span.dataset.mathStyle === "display";
    try {
      katex.render(span.textContent, span, {
        displayMode: display,
        throwOnError: false,
      });
    } catch (_) {
      /* 失敗時は元テキストのまま残す */
    }
  });
}

let mermaidReady = false;
function renderMermaid(root) {
  if (typeof mermaid === "undefined") return;
  const blocks = root.querySelectorAll("pre > code.language-mermaid");
  if (blocks.length === 0) return;

  if (!mermaidReady) {
    // deterministicIds: 同一ミリ秒で複数図を描いても id が衝突しないようにする。
    mermaid.initialize({
      startOnLoad: false,
      theme: mermaidTheme(),
      deterministicIds: true,
    });
    mermaidReady = true;
  }

  blocks.forEach((code) => {
    const pre = code.parentElement;
    const div = document.createElement("div");
    div.className = "mermaid";
    div.textContent = code.textContent;
    pre.replaceWith(div);
  });

  // mermaid.run は非同期。同期例外と Promise reject の両方を握りつぶす。
  try {
    const result = mermaid.run({ nodes: root.querySelectorAll(".mermaid") });
    if (result && typeof result.catch === "function") {
      result.catch(() => {});
    }
  } catch (_) {
    /* 図の描画失敗は無視 */
  }
}

function mermaidTheme() {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "default";
}

// ---- ライブリロード ----

function resync() {
  loadTree();
  if (state.currentPath) openFile(state.currentPath);
}

function connectEvents() {
  const source = new EventSource("/events");
  source.addEventListener("reload", async (e) => {
    // 表示中ファイルの変更、または "*"(Lagged=取りこぼし)なら本文を先に再取得。
    // 削除されていれば openFile 内で currentPath がクリアされる。
    if (state.currentPath && (e.data === "*" || e.data === state.currentPath)) {
      await openFile(state.currentPath);
    }
    // その後ツリーを更新。表示中ファイルが消えていれば先頭ファイルを自動で開く。
    loadTree();
  });
  // 再接続成功時は、切断中の変更取りこぼしに備えて再同期する。
  source.onopen = () => resync();
  source.onerror = () => {
    // EventSource が自動再接続する。復帰時は onopen で再同期するので握りつぶす。
  };
}

// ---- 起動 ----

loadTree();
connectEvents();

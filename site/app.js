"use strict";

const $ = id => document.getElementById(id);
const D = window.DATA;
const state = { id: "canned", k: "1", panel: 0, outage: "none" };
const number = n => n === null ? "—" : Number(n).toLocaleString("en-US");
const repo = "https://github.com/symonyeal/Supplier-Resilience-Lab";

function node(tag, text, cls) {
  const e = document.createElement(tag);
  if (text !== undefined) e.textContent = text;
  if (cls) e.className = cls;
  return e;
}

function option(value, text) {
  const e = node("option", text);
  e.value = value;
  return e;
}

function current() {
  const c = D.cases.find(c => c.id === state.id);
  const m = c.modes[state.k];
  const p = m.panels[state.panel];
  const o = c.outages.find(o => o.id === state.outage);
  return { c, m, p, o, result: p ? p.outages[o.id] : null };
}

function name(c, v) { return c.suppliers.find(s => s.id === v).name; }

function metric(value, label) {
  const e = node("div", undefined, "metric");
  e.append(node("strong", value), node("span", label));
  return e;
}

function renderMatrix() {
  const { c, p, o } = current();
  const query = $("supplier-search").value.toLocaleLowerCase();
  const head = node("tr");
  for (const title of ["Supplier", ...c.products]) {
    const th = node("th", title);
    th.scope = "col";
    head.append(th);
  }
  $("matrix-head").replaceChildren(head);
  const rows = c.suppliers.filter(s => s.name.toLocaleLowerCase().includes(query)).map(s => {
    const selected = p && p.members.includes(s.id);
    const out = o.failed.includes(s.id);
    const row = node("tr", undefined, out ? "out" : selected ? "in-panel" : "");
    const label = node("td", s.name);
    label.append(node("small", `${s.id}${selected ? " · Selected" : ""}${out ? " · Outage" : ""}`));
    row.append(label);
    for (const q of s.qualifications) {
      const cell = node("td");
      const mark = node("span", q ? "Q" : "—", q ? "q" : "");
      mark.setAttribute("aria-label", q ? "Qualified" : "No recorded qualification");
      cell.append(mark);
      row.append(cell);
    }
    return row;
  });
  if (!rows.length) {
    const row = node("tr");
    const cell = node("td", "No suppliers match this search.");
    cell.colSpan = c.products.length + 1;
    row.append(cell);
    rows.push(row);
  }
  $("matrix-body").replaceChildren(...rows);
}

function render() {
  const { c, m, p, o, result } = current();
  for (const b of $("case-tabs").children) b.setAttribute("aria-pressed", String(b.dataset.case === c.id));
  for (const radio of document.querySelectorAll('[name="budget"]')) radio.checked = radio.value === state.k;
  $("case-title").textContent = c.title;
  $("case-note").textContent = m.count
    ? `${number(m.count)} minimal panel${m.count === 1 ? "" : "s"}; the smallest has ${m.minimum_size}.`
    : `No panel: ${m.infeasibility[0]} has too few qualified suppliers.`;
  $("panel").replaceChildren(...m.panels.map((p, i) => option(i,
    `${i === 0 ? "Minimum panel" : `Alternative ${i + 1}`} · ${p.members.length} suppliers`)));
  if (!p) $("panel").append(option("", "No feasible panel"));
  $("panel").value = p ? String(state.panel) : "";
  $("panel").disabled = !p;
  $("shortlist-note").textContent = `${m.panels.length} of ${number(m.count)} shown.`;
  $("outage").replaceChildren(...c.outages.map(o => option(o.id, o.label)));
  $("outage").value = o.id;
  $("outage").disabled = !p;
  $("outage-note").textContent = o.failed.length
    ? `Hypothetical: ${o.failed.map(v => name(c, v)).join("; ")} fail${o.failed.length === 1 ? "s" : ""}.`
    : "Hypothetical stress test.";
  $("result-metrics").replaceChildren(
    metric(number(m.minimum_size), "Minimum panel size"),
    metric(number(m.count), "Minimal panels"),
    metric(m.essential === null ? "—" : Object.keys(m.essential).length, "Essential suppliers"));
  $("panel-results").hidden = !p;
  $("infeasible").hidden = !!p;
  $("coverage-status").classList.toggle("lost", !result || !result.covered);
  if (p) {
    $("coverage-status").textContent = result.covered ? "Coverage retained" : "Coverage lost";
    $("status-tag").textContent = `${c.products.length - result.uncovered.length} / ${c.products.length} products`;
    $("selected-suppliers").replaceChildren(...p.members.map(v => {
      const failed = o.failed.includes(v);
      const item = node("div", undefined, `supplier${failed ? " failed" : ""}`);
      item.append(node("strong", name(c, v)), node("span", failed ? "Failed" : "Available"));
      return item;
    }));
    $("coverage-rows").replaceChildren(...c.products.map((product, j) => {
      const vendors = result.suppliers[j];
      const row = node("tr");
      row.append(node("td", product), node("td", vendors.length ? vendors.map(v => name(c, v)).join("; ") : "None"),
        node("td", vendors.length ? "✓ Covered" : "× Uncovered", vendors.length ? "ok" : "missing"));
      return row;
    }));
  } else {
    $("coverage-status").textContent = "No resilient panel exists";
    $("status-tag").textContent = "Matrix witness";
    const [product] = m.infeasibility;
    const j = c.products.indexOf(product);
    $("infeasible").replaceChildren(node("strong", `Blocked by: ${product}`),
      node("span", `${c.product_sizes[j]} qualified; k = ${state.k} needs ${Number(state.k) + 1}. Only a new qualification fixes this.`));
  }
  $("alternative-count").textContent = `${m.panels.length} shown`;
  $("alternatives").replaceChildren(...m.panels.map((p, i) => {
    const b = node("button", undefined, "alternative");
    b.type = "button";
    b.dataset.panel = i;
    b.setAttribute("aria-pressed", String(i === state.panel));
    const heading = node("strong", i === 0 ? "Minimum panel" : `Alternative ${i + 1}`);
    heading.append(node("span", `${p.members.length} suppliers`));
    b.append(heading, node("span", p.members.map(v => name(c, v)).join(" · ")),
      node("span", p.outages[o.id].covered ? "Survives this outage" : "Fails this outage"));
    b.addEventListener("click", () => {
      state.panel = i;
      render();
      document.querySelector(`[data-panel="${i}"]`).focus({ preventScroll: true });
    });
    return b;
  }));
  if (!p) $("alternatives").append(node("p", "None. Select k = 0.", "fine"));
  renderMatrix();
}

function download(filename, content, type) {
  const url = URL.createObjectURL(new Blob([content], { type }));
  const a = node("a");
  a.href = url;
  a.download = filename;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function escape(text) {
  return String(text).replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
}

function report() {
  const { c, m, p, o, result } = current();
  const e = escape;
  const names = vs => vs.map(v => e(name(c, v))).join("; ");
  const status = result ? result.covered ? "Coverage retained" : "Coverage lost" : "No resilient panel exists";
  const rows = c.products.map((product, j) => `<tr><td>${e(product)}</td><td>${result ? names(result.suppliers[j]) || "None" : "—"}</td></tr>`).join("");
  const certificates = p ? p.members.map(v => {
    const [product, F] = p.witnesses[v];
    return `<li>${e(name(c, v))}: sole cover of ${e(product)}${F.length ? ` if ${names(F)} fails` : ""}.</li>`;
  }).join("") : `<li>Infeasible: ${e(m.infeasibility[0])} has too few qualified suppliers.</li>`;
  const provenance = Object.entries(D.provenance).filter(([k]) => k !== "snapshot").map(([k, v]) => `<dt>${e(k)}</dt><dd>${e(v)}</dd>`).join("");
  return `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${e(c.title)}: report</title><style>body{max-width:900px;margin:45px auto;padding:0 20px;font:16px/1.6 system-ui;color:#202a27}h1{line-height:1.2}table{width:100%;border-collapse:collapse}td,th{text-align:left;border-bottom:1px solid #ccc;padding:12px}dd{margin:0 0 12px;font:13px/1.6 monospace;overflow-wrap:anywhere}dt{font-weight:600}p{max-width:75ch}</style>`
    + `<h1>${e(status)}: ${e(c.title)}</h1>`
    + `<p>Panel: ${p ? names(p.members) : "none"}. Survives any ${e(state.k)} failure. Outage tested: ${o.failed.length ? names(o.failed) : "none"} (hypothetical). Minimum ${e(m.minimum_size ?? "—")}; ${e(m.count)} minimal panels in total.</p>`
    + `<table><thead><tr><th>Product</th><th>Surviving suppliers</th></tr></thead><tbody>${rows}</tbody></table>`
    + `<h2>Certificates</h2><ul>${certificates}</ul><p>Every panel certified minimal from the matrix; exhaustion check ${e(m.checks.milp_exhaustion)}.</p>`
    + `<h2>Scope</h2><p>Qualification coverage only: not capacity, price, delivery or plant independence. Not a purchasing recommendation.</p>`
    + `<p>USDA QBL retrieved ${e(D.provenance.snapshot.retrieved_utc)}. <a href="${e(D.provenance.snapshot.url)}">Source</a> · <a href="${repo}">Code</a>.</p><dl>${provenance}</dl></html>`;
}

const ROUTES = { berge: "Berge", fkb: "FK-B", milp: "MILP enumeration" };
const O = { class: "", rule: "", k: "", outcome: "", product: "", i: 0 };
const kind = x => x.min.status !== "optimal" ? "infeasible" : x.count_exact === null ? "bound" : "exact";
const kinds = { exact: "Exact count", bound: "Lower bound", infeasible: "Infeasible" };
const count = x => kind(x) === "infeasible" ? "0" : x.count_exact !== null ? number(x.count_exact) : `≥ ${number(x.count_lower_bound)}`;
const basket = x => `${x.class} · ${x.rule} · ${x.size} products`;
const supplier = v => D.evidence.names[v] || v;
const seconds = t => t === null ? "—" : t < 1 ? `${Number((t * 1000).toPrecision(2))} ms` : `${t.toFixed(2)} s`;
const token = name => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

function matches(x) {
  const q = O.product.toLocaleLowerCase();
  return (!O.class || x.class === O.class) && (!O.rule || x.rule === O.rule)
    && (!O.k || String(x.k) === O.k) && (!O.outcome || kind(x) === O.outcome)
    && (!q || x.P.some(p => p.toLocaleLowerCase().includes(q)));
}

function selectOutcome(i) {
  O.i = i;
  renderOutcomes();
}

function renderChart(R, shown) {
  const div = $("outcome-chart");
  if (!window.Plotly) {
    div.replaceChildren(node("p", "The chart library did not load. The table lists every instance.", "fine"));
    return;
  }
  const F = R.map((x, i) => i).filter(i => kind(R[i]) !== "infeasible");
  const trace = (I, name, marker) => ({
    type: "scatter", mode: "markers", name, x: I.map(i => R[i].V), y: I.map(i => R[i].count_lower_bound),
    customdata: I, text: I.map(i => `${basket(R[i])}<br>k = ${R[i].k} · minimum ${R[i].min.size}<br>${R[i].V} eligible · ${count(R[i])} minimal panels`),
    hovertemplate: "%{text}<extra></extra>", marker });
  const accent = token("--accent");
  const traces = [
    trace(F.filter(i => !shown.has(i)), "Outside filter", { color: token("--border"), size: 8 }),
    trace(F.filter(i => shown.has(i) && kind(R[i]) === "exact"), "Exact count", { color: accent, size: 9, opacity: .85 }),
    trace(F.filter(i => shown.has(i) && kind(R[i]) === "bound"), "Lower bound (run stopped)", { color: accent, size: 12, symbol: "triangle-up-open", line: { width: 2 } }),
    trace(F.filter(i => i === O.i), "Selected", { color: "rgba(0,0,0,0)", size: 20, line: { color: token("--text"), width: 2 } })];
  const axis = (title, T) => ({ type: "log", title: { text: title }, gridcolor: token("--border"), zeroline: false,
    tickfont: { size: 12 }, tickvals: T, ticktext: T.map(number) });
  Plotly.react(div, traces, {
    margin: { l: 64, r: 12, t: 8, b: 56 }, paper_bgcolor: "rgba(0,0,0,0)", plot_bgcolor: "rgba(0,0,0,0)",
    font: { family: token("--font"), color: token("--muted"), size: 13 }, hovermode: "closest",
    legend: { orientation: "h", x: 0, y: 1.02, yanchor: "bottom" },
    xaxis: axis("Eligible suppliers |V| (log scale)", [3, 5, 10, 20, 50, 100]),
    yaxis: axis("Minimal panels (log scale)", [1, 10, 100, 1000, 10000]) },
    { displayModeBar: false, responsive: true });
  if (!div.dataset.bound) {
    div.on("plotly_click", e => selectOutcome(e.points[0].customdata));
    div.dataset.bound = "1";
  }
}

function renderRecord(R) {
  const x = R[O.i];
  if (x === undefined) {
    $("record").replaceChildren(node("p", "No instance matches these filters.", "fine"));
    return;
  }
  const head = node("div", undefined, "box-heading");
  head.append(node("span", `Instance ${O.i + 1} of ${R.length} · seed 1`, "small-label"), node("span", kinds[kind(x)], "tag"));
  const title = node("h3", `${basket(x)}, k = ${x.k}`);
  title.id = "record-title";
  const metrics = node("div", undefined, "result-metrics");
  metrics.append(metric(number(x.V), "Eligible suppliers"), metric(number(x.min.size), "Minimum panel size"),
    metric(count(x), { exact: "Minimal panels · exact", bound: "Minimal panels · lower bound", infeasible: "No feasible panel" }[kind(x)]));
  const list = items => { const u = node("ul"); u.append(...items.map(([t, c]) => node("li", t, c))); return u; };
  const parts = [head, title, metrics, node("h4", "Qualified suppliers per product"),
    list(x.P.map((p, j) => [`${p}: ${x.E_p[j]}`, x.E_p[j] <= x.k ? "scarce" : ""]))];
  if (kind(x) === "infeasible") {
    const [product] = x.infeasibility;
    const n = x.E_p[x.P.indexOf(product)];
    parts.push(node("h4", "Infeasibility witness"),
      node("p", `${product}: ${n} qualified; k = ${x.k} needs ${x.k + 1}.`, "fine"));
  } else {
    const E = Object.entries(x.essential);
    parts.push(node("h4", "Minimum panel"), list(x.min.panel.map(v => [supplier(v)])),
      node("h4", "Essential suppliers"),
      E.length ? list(E.map(([v, [p, F]]) => [`${supplier(v)}: sole cover of ${p}${F.length ? ` if ${F.map(supplier).join("; ")} fails` : ""}`]))
        : node("p", "None.", "fine"));
  }
  const table = node("table");
  const tr = node("tr");
  for (const h of ["Route", "Status", "Panels", "First", "Total"]) { const th = node("th", h); th.scope = "col"; tr.append(th); }
  table.append(node("thead"), node("tbody"));
  table.tHead.append(tr);
  table.tBodies[0].append(...x.runs.map(r => {
    const row = node("tr");
    row.append(node("td", ROUTES[r.method]), node("td", r.status), node("td", number(r.count)), node("td", seconds(r.t_first)), node("td", seconds(r.t_total)));
    return row;
  }));
  const scroll = node("div", undefined, "table-scroll");
  scroll.append(table);
  const s = x.sensitivity;
  parts.push(node("h4", "Runs"), scroll, node("p",
    `Exhaustion check: ${x.milp_exhaustion || "not run"}. Routes agree: ${x.complete_routes_agree === null ? "n/a" : x.complete_routes_agree ? "yes" : "no"}.`
    + (s ? ` Minimum with merged mappings: ${number(s.merged.min)}.` : ""), "fine"));
  $("record").replaceChildren(...parts);
}

function renderOutcomes() {
  const R = D.evidence.records;
  const I = R.map((x, i) => i).filter(i => matches(R[i]));
  if (!I.includes(O.i)) O.i = I.length ? I[0] : -1;
  $("outcome-count").textContent = `${I.length} of ${R.length} shown`;
  $("outcome-rows").replaceChildren(...I.map(i => {
    const x = R[i];
    const row = node("tr", undefined, i === O.i ? "selected" : "");
    const cell = node("td");
    const b = node("button", basket(x));
    b.type = "button";
    b.dataset.record = i;
    b.setAttribute("aria-pressed", String(i === O.i));
    b.addEventListener("click", () => { selectOutcome(i); document.querySelector(`[data-record="${i}"]`).focus({ preventScroll: true }); });
    cell.append(b, node("small", x.P.join("; ")));
    row.append(cell, node("td", String(x.k)), node("td", number(x.V)), node("td", number(x.min.size)),
      node("td", count(x), { bound: "bound", infeasible: "infeasible-cell" }[kind(x)] || ""));
    return row;
  }));
  if (!I.length) {
    const row = node("tr"), cell = node("td", "No instances match these filters.");
    cell.colSpan = 5;
    row.append(cell);
    $("outcome-rows").append(row);
  }
  $("download-record").disabled = O.i < 0;
  renderChart(R, new Set(I));
  renderRecord(R);
}

function csv(R) {
  const q = v => /[",\n]/.test(String(v ?? "")) ? `"${String(v).replace(/"/g, '""')}"` : String(v ?? "");
  const head = ["class", "rule", "size", "k", "eligible", "outcome", "minimum", "count_exact", "count_lower_bound", "essential", "products"];
  return [head, ...R.map(x => [x.class, x.rule, x.size, x.k, x.V, kind(x), x.min.size, x.count_exact, x.count_lower_bound,
    x.essential === null ? "" : Object.keys(x.essential).length, x.P.join("; ")])].map(r => r.map(q).join(",")).join("\n") + "\n";
}

function initialiseOutcomes() {
  const B = D.evidence;
  const R = B.records;
  const distinct = key => [...new Set(R.map(x => x[key]))];
  $("f-class").append(...distinct("class").map(v => option(v, v)));
  $("f-rule").append(...distinct("rule").map(v => option(v, v)));
  $("outcomes-summary").textContent = `All ${R.length} saved results, re-certified from the matrix; not rerun.`;
  $("chart-note").textContent = `Spearman ρ = ${B.spearman_V_count}, n = ${B.feasible} feasible. ${R.length - B.feasible} infeasible: table only. △ lower bound (run stopped at ${B.provenance.limits.timeout_s} s). USDA QBL, seed 1, code ${B.provenance.code_rev.slice(0, 7)}.`;
  for (const [id, key] of [["f-class", "class"], ["f-rule", "rule"], ["f-k", "k"], ["f-outcome", "outcome"]])
    $(id).addEventListener("change", e => { O[key] = e.target.value; renderOutcomes(); });
  $("f-product").addEventListener("input", e => { O.product = e.target.value; renderOutcomes(); });
  $("f-reset").addEventListener("click", () => {
    Object.assign(O, { class: "", rule: "", k: "", outcome: "", product: "", i: 0 });
    for (const id of ["f-class", "f-rule", "f-k", "f-outcome", "f-product"]) $(id).value = "";
    renderOutcomes();
  });
  $("download-outcomes").addEventListener("click", () => download("seed1-outcomes.csv", csv(R.filter(matches)), "text/csv;charset=utf-8"));
  $("download-record").addEventListener("click", () => {
    const x = R[O.i];
    const used = new Set([...(x.min.panel || []), ...Object.keys(x.essential || {})]);
    const record = { ...x, names: Object.fromEntries([...used].map(v => [v, supplier(v)])), provenance: B.provenance };
    download(`seed1-${x.class}-${x.rule}-s${x.size}-k${x.k}.json`, JSON.stringify(record, null, 1) + "\n", "application/json");
  });
  renderOutcomes();
}

function initialise() {
  const lead = D.cases[0];
  $("hero-products").replaceChildren(...lead.products.map(p => node("span", p)));
  $("hero-eligible").textContent = number(lead.eligible);
  $("hero-minimum").textContent = number(lead.modes["1"].minimum_size);
  $("hero-count").textContent = number(lead.modes["1"].count);
  const counts = D.cases.map(c => c.modes["1"].count);
  $("lab-title").textContent = `Resilient options range from ${number(Math.max(...counts))} panels to ${Math.min(...counts) ? number(Math.min(...counts)) : "none"}.`;
  $("hero-source").textContent = `USDA QBL, ${D.provenance.snapshot.retrieved_utc.slice(0, 10)} · all panels certified`;
  $("case-tabs").replaceChildren(...D.cases.map((c, i) => {
    const b = node("button", `${String(i + 1).padStart(2, "0")} / ${c.title}`);
    b.type = "button";
    b.dataset.case = c.id;
    b.addEventListener("click", () => {
      Object.assign(state, { id: c.id, panel: 0, outage: "none" });
      $("supplier-search").value = "";
      render();
    });
    return b;
  }));
  for (const radio of document.querySelectorAll('[name="budget"]')) radio.addEventListener("change", () => {
    Object.assign(state, { k: radio.value, panel: 0 }); render();
  });
  $("panel").addEventListener("change", e => { state.panel = Number(e.target.value); render(); });
  $("outage").addEventListener("change", e => { state.outage = e.target.value; render(); });
  $("supplier-search").addEventListener("input", renderMatrix);
  $("reset").addEventListener("click", () => {
    Object.assign(state, { id: "canned", k: "1", panel: 0, outage: "none" });
    $("supplier-search").value = "";
    render();
  });
  $("download-report").addEventListener("click", () => download(`${state.id}-k${state.k}-decision.html`, report(), "text/html;charset=utf-8"));
  $("download-family").addEventListener("click", () => {
    const { c, m } = current();
    const family = { ...m.family, ...D.provenance, kind: "enumeration", names: Object.fromEntries(c.suppliers.map(s => [s.id, s.name])) };
    delete family.snapshot;
    download(`${c.id}-k${state.k}-family.json`, JSON.stringify(family, null, 1) + "\n", "application/json");
  });
  const B = D.evidence;
  const blocked = B.records.filter(x => x.k === 1 && x.rule !== "scarce" && x.min.status !== "optimal");
  const sizes = B.records.filter(x => x.k === 1 && x.rule !== "scarce" && x.min.status === "optimal").map(x => x.min.size);
  const single = blocked.filter(x => Math.min(...x.E_p) === 1).length;
  $("h-fail").textContent = blocked.length;
  $("h-primary").textContent = B.primary;
  $("resilient-count").textContent = `${blocked.length} / ${B.primary}`;
  $("finding-note").textContent = `${single === blocked.length ? "In every one" : `In ${single}`}, a product has one qualified supplier. In the other ${sizes.length}, ${Math.min(...sizes)}–${Math.max(...sizes)} suppliers suffice.`;
  $("benchmark-summary").textContent = `${B.instances} instances, seed 1. Historical timings; not rerun.`;
  const best = B.routes.reduce((a, r) => r.complete > a.complete ? r : a);
  $("benchmark-caption").textContent = `${ROUTES[best.method]} finishes the most enumerations: ${best.complete} of ${best.runs}.`;
  $("benchmark-rows").replaceChildren(...B.routes.map(r => {
    const row = node("tr");
    const time = node("td", `${(r.first_median_s * 1000).toPrecision(2)} ms`);
    time.append(node("small", `n = ${r.first_n}`));
    row.append(node("td", ROUTES[r.method]), node("td", `${r.complete} / ${r.runs}`), time);
    return row;
  }));
  $("benchmark-note").textContent = `Code ${B.provenance.code_rev.slice(0, 7)} · ${B.provenance.limits.timeout_s} s limit · ${number(B.provenance.limits.cap)} output cap · stopped runs censored · first-panel medians over runs that emitted one.`;
  const p = D.provenance;
  const entries = { "QBL retrieved (UTC)": p.snapshot.retrieved_utc, "Data SHA-256": p.data_sha256,
    "Mappings SHA-256": p.mappings_sha256, "Case build code": p.code_rev + (p.dirty ? " (working changes)" : ""),
    "Source SHA-256": p.code_sha256, "Thesis revision": p.thesis_rev,
    "Software / schema": `srl ${p.software_version} / ${D.schema_version}`,
    "Snapshot identity": `Row IDs; ${p.identity_candidates} unresolved identity candidates` };
  $("provenance").replaceChildren(...Object.entries(entries).flatMap(([k, v]) => [node("dt", k), node("dd", v)]));
  render();
  initialiseOutcomes();
}

if (D) initialise();
else $("coverage-status").textContent = "Case data could not be loaded. Download the source to reproduce it.";

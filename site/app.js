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
  $("case-note").textContent = c.note;
  $("panel").replaceChildren(...m.panels.map((p, i) => option(i,
    `${i === 0 ? "Minimum panel" : `Alternative ${i + 1}`} · ${p.members.length} suppliers`)));
  if (!p) $("panel").append(option("", "No feasible panel"));
  $("panel").value = p ? String(state.panel) : "";
  $("panel").disabled = !p;
  $("shortlist-note").textContent = `${m.panels.length} shown of ${number(m.count)} minimal panels. Full family available below.`;
  $("outage").replaceChildren(...c.outages.map(o => option(o.id, o.label)));
  $("outage").value = o.id;
  $("outage").disabled = !p;
  $("outage-note").textContent = o.failed.length
    ? `Hypothetical outage: ${o.failed.map(v => name(c, v)).join("; ")}. No shared ownership is assumed.`
    : "The outage is a hypothetical stress test, not an observed event.";
  $("result-metrics").replaceChildren(
    metric(number(m.minimum_size), "Minimum panel size"),
    metric(number(m.count), "Minimal panels · exact count"),
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
      item.append(node("strong", name(c, v)), node("span", failed ? "Unavailable in this stress test" : "Available in this stress test"));
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
    $("coverage-status").textContent = "Coverage requirement is infeasible";
    $("status-tag").textContent = "Matrix witness";
    const [product] = m.infeasibility;
    const j = c.products.indexOf(product);
    $("infeasible").replaceChildren(node("strong", `Blocked by: ${product}`),
      node("span", `This product has ${c.product_sizes[j]} qualified supplier(s). Surviving any ${state.k} failure(s) requires at least ${Number(state.k) + 1}. Choosing a larger panel cannot remove this qualification bottleneck.`));
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
      node("span", p.outages[o.id].covered ? "Coverage retained under this outage" : "Coverage lost under this outage"));
    b.addEventListener("click", () => {
      state.panel = i;
      render();
      document.querySelector(`[data-panel="${i}"]`).focus({ preventScroll: true });
    });
    return b;
  }));
  if (!p) $("alternatives").append(node("p", "No feasible alternatives exist under this requirement. Inspect the blocking product above, or select k = 0.", "fine"));
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
  const status = result ? result.covered ? "Coverage retained" : "Coverage lost" : "Requirement infeasible";
  const rows = c.products.map((product, j) => `<tr><td>${e(product)}</td><td>${result ? result.suppliers[j].map(v => e(name(c, v))).join("; ") || "None" : "No feasible panel"}</td></tr>`).join("");
  const witnesses = p ? p.members.map(v => {
    const [product, F] = p.witnesses[v];
    return `<li>${e(name(c, v))}: removing this supplier loses ${e(product)} when ${F.length ? F.map(w => e(name(c, w))).join("; ") : "no other supplier"} fails.</li>`;
  }).join("") : `<li>Infeasibility witness: ${e(JSON.stringify(m.infeasibility))}</li>`;
  const provenance = Object.entries(D.provenance).filter(([k]) => k !== "snapshot").map(([k, v]) => `<dt>${e(k)}</dt><dd>${e(v)}</dd>`).join("");
  return `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${e(c.title)} — decision report</title><style>body{max-width:900px;margin:45px auto;padding:0 20px;font:16px/1.6 system-ui;color:#202a27}h1{line-height:1.2}table{width:100%;border-collapse:collapse}td,th{text-align:left;border-bottom:1px solid #ccc;padding:12px}dd{margin:0 0 12px;font:13px/1.6 monospace;overflow-wrap:anywhere}dt{font-weight:600}p{max-width:75ch}</style><h1>Supplier Resilience Lab: ${e(c.title)}</h1><p><strong>${e(status)}</strong>. Original panel requirement: survive any ${e(state.k)} supplier failure(s). Outage checked here: ${o.failed.length ? o.failed.map(v => e(name(c, v))).join("; ") : "none"}.</p><p>Selected panel: ${p ? p.members.map(v => e(name(c, v))).join("; ") : "none"}. Minimum panel size: ${e(m.minimum_size ?? "infeasible")}. Complete family: ${e(m.count)} minimal panels.</p><table><caption>Coverage under the selected outage</caption><thead><tr><th>Product</th><th>Surviving qualified panel suppliers</th></tr></thead><tbody>${rows}</tbody></table><h2>Original panel certificates</h2><ul>${witnesses}</ul><p>All exported panels were independently checked for minimality; MILP exhaustion returned ${e(m.checks.milp_exhaustion)}. The selected outage is a hypothetical stress test. It is not evidence of common ownership or an actual event.</p><h2>Scope and source</h2><p>Qualification coverage only. Capacity, price, delivery performance and plant independence are not established. Supplier IDs are snapshot row identities. This report is a research result, not a purchasing recommendation.</p><p>QBL retrieved ${e(D.provenance.snapshot.retrieved_utc)}. <a href="${e(D.provenance.snapshot.url)}">USDA source</a>. <a href="${repo}">Code and methods</a>.</p><dl>${provenance}</dl></html>`;
}

function initialise() {
  const lead = D.cases[0];
  $("hero-products").replaceChildren(...lead.products.map(p => node("span", p)));
  $("hero-eligible").textContent = number(lead.eligible);
  $("hero-minimum").textContent = number(lead.modes["1"].minimum_size);
  $("hero-count").textContent = number(lead.modes["1"].count);
  $("hero-source").textContent = `USDA QBL · ${D.provenance.snapshot.retrieved_utc.slice(0, 10)} snapshot · all panels certified`;
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
  $("resilient-count").textContent = `${B.resilient} / ${B.primary}`;
  $("benchmark-summary").textContent = `${B.instances} real-data instances from the frozen seed-1 suite. These historical measurements are distinct from the interactive case builds.`;
  const titles = { berge: "Berge", fkb: "FK-B", milp: "MILP enumeration" };
  $("benchmark-rows").replaceChildren(...B.routes.map(r => {
    const row = node("tr");
    const time = node("td", `${(r.first_median_s * 1000).toPrecision(2)} ms`);
    time.append(node("small", `n = ${r.first_n} emitting runs`));
    row.append(node("td", titles[r.method]), node("td", `${r.complete} / ${r.runs}`), time);
    return row;
  }));
  $("benchmark-note").textContent = `Seed 1 · code ${B.provenance.code_rev.slice(0, 7)} · ${B.provenance.limits.timeout_s} s per enumeration · ${number(B.provenance.limits.cap)} output cap (Berge may count beyond it) · child-process timings.`;
  const p = D.provenance;
  const entries = { "QBL retrieved (UTC)": p.snapshot.retrieved_utc, "Data SHA-256": p.data_sha256,
    "Mappings SHA-256": p.mappings_sha256, "Case build code": p.code_rev + (p.dirty ? " (working changes)" : ""),
    "Source SHA-256": p.code_sha256, "Thesis revision": p.thesis_rev,
    "Software / schema": `srl ${p.software_version} / ${D.schema_version}`,
    "Snapshot identity": `Row IDs; ${p.identity_candidates} unresolved identity candidates` };
  $("provenance").replaceChildren(...Object.entries(entries).flatMap(([k, v]) => [node("dt", k), node("dd", v)]));
  render();
}

if (D) initialise();
else $("coverage-status").textContent = "Case data could not be loaded. Download the source to reproduce it.";

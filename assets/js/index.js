function isValidUUID(uuid) {
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
    return uuidRegex.test(uuid);
}

const SOLVER = new URLSearchParams(window.location.search).get('solver');
const RUN = new URLSearchParams(window.location.search).get('run');
const RUN_MODE = (SOLVER !== null) && (RUN !== null);

if (RUN_MODE && !isValidUUID(SOLVER)) {
    alert('Invalid solver_uuid');
    window.location.href = '/';
}

if (RUN_MODE && !isValidUUID(RUN)) {
    alert('Invalid run_uuid');
    window.location.href = '/';
}

if (RUN_MODE) {
    document.querySelector('#breadcrumb-solver').href += `?solver=${SOLVER}`;

} else {
    document.querySelectorAll(".run-mode-only").forEach((e) => e.remove());
}

const apiBase = '/api/';
const apiTags = apiBase + 'tags';
const apiInstances = apiBase + 'instances/list';
const apiSolverRunList = apiBase + `solver_run/list?solver=${SOLVER}&run=${RUN}`;

let tags = null;

let filterOptions = {
    direction: "asc",
    order_by: "id",
    current_page: 1,
    tag: null
};

function createTagElement(data, with_counts = false) {
    const tag = document.createElement('span');
    tag.className = `tag tagstyle${data.style}`;

    if (filterOptions.tag !== null) {
        tag.classList.add(
            filterOptions.tag == data.tid
                ? "active" : "inactive");
    }

    tag.innerText = data.name +
        (with_counts ? ` (${data.num_instances})` : '');

    return tag;
}

function populateTags() {
    const tagSelect = document.querySelector('#tag');
    tagSelect.innerHTML = '<option value="none">All</option>';

    for (let tid in tags) {
        let option = document.createElement('option');
        option.value = tid;
        option.innerText = tags[tid].name;
        tagSelect.appendChild(option);
    }
}

function buildFilter() {
    let filter = {
        sort_direction: filterOptions["direction"],
        sort_by: filterOptions["order_by"],
    }

    if (RUN_MODE) {
        filter["run"] = RUN;
        filter["solver"] = SOLVER;
    }

    if (document.querySelector("#tag").value != "none") {
        filter["tag"] = document.querySelector("#tag").value;
    }

    document.querySelectorAll(".form-control").forEach((e) => {
        function update(key, value) {
            if (value == "none") { return; }

            if (parseInt(value)) {
                value = parseInt(value);
            }

            filter[key] = value;
        }

        if (e.id.startsWith("constr_")) {
            update(e.id.replace("constr_", ""), e.value)

        } else if (e.id.startsWith("bool_constr_")) {
            update(e.id.replace("bool_constr_", ""), e.value);
        }
    });

    return filter;
}

function fetchData(include_tags = false, include_max_values = false) {
    let filters = buildFilter();

    // TODO: fix document.querySelector("#reset_filters").disabled = (filters.length == 2);

    filters["page"] = filterOptions["current_page"];
    filters["limit"] = 100;

    if (include_tags) {
        filters["include_tag_list"] = true;
    }

    if (include_max_values) {
        filters["include_max_values"] = true;
    }

    {
        const cls = "table-primary";
        document.querySelectorAll(`#instances th.${cls}`).forEach((e) => { e.classList.remove(cls) });
        document.querySelector(`#instances #header_${filterOptions.order_by}`).classList.add(cls);
    }

    fetch(`${apiInstances}`, {
        method: 'POST',
        headers: {
            'Accept': 'application/json',
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(filters)
    })
        .then(response => response.json())
        .then(data => {
            if (include_tags) {
                tags = {};
                data.tags.forEach(tag => {
                    tags[tag.tid] = tag;
                });
                populateTags();
            }

            if (include_max_values) {
                populateMaxValues(data.max_values);
            }

            populateTable(data.results);

            let download_btn = document.querySelector("#download_list");
            download_btn.innerText = `Download list of ${data.total_matches} matches`;
            download_btn.disabled = (data.total_matches == 0);

            setupPagination(data.options.page, Math.ceil(data.total_matches / data.options.limit));
        })
        .catch(error => console.error('Error fetching data:', error));
}

function populateTable(instances) {
    const tableHeader = document.querySelector('#instances thead tr');
    const tableBody = document.querySelector('#instances tbody');
    tableBody.innerHTML = '';
    instances.forEach(ins => {
        const row = document.createElement('tr');

        ins.regular = (ins.min_deg == ins.max_deg);

        function create_td(value, fmt, order_by_key) {
            let elem = document.createElement("td");
            elem.classList.add(fmt);

            if (filterOptions.order_by == order_by_key) {
                elem.classList.add("table-primary");
            }

            elem.innerText = value;
            row.appendChild(elem);

            return elem;
        }

        function add_td(key, fmt = "num", if_unknown = "?", order_by = null) {
            let value = ins[key];

            if (value === null || value === undefined) {
                value = if_unknown;
            } else {
                if (fmt == "num") {
                    if (key != "iid" && value > 1e4) {
                        if (value < 1000) { }
                        else if (value < 1e6) {
                            value = Math.round(value / 1000) + "K";
                        } else {
                            value = (value / 1e6).toFixed(2) + "M";

                        }
                    }
                } else {
                    value = value ? "✅" : "❌";
                }
            }

            const col_idx = row.querySelectorAll("td").length;
            let elem = create_td(value, fmt, order_by === null ? key : order_by);
            const header_elem = tableHeader.querySelectorAll("th")[col_idx];
            if (header_elem.classList.contains("group-begin")) {
                elem.classList.add("group-begin");                
            }
        }

        add_td("iid", "num", null, "id");

        {
            let name_elem = document.createElement("td");
            name_elem.innerHTML = `<span class="name">${ins.name}</span>
                        <a href="${apiBase}instances/download/${ins.iid}" alt="Download Instance ${ins.iid}">⬇️</a>
                        <span class="tags"></span><p class="desc">${ins.description}</p>`;

            if (filterOptions.order_by == "name") {
                name_elem.classList.add("table-primary");
            }

            row.appendChild(name_elem);
        }


        add_td("nodes");
        add_td("edges");
        add_td("best_score", "num", "❓");
        add_td("min_deg");
        add_td("max_deg");
        add_td("regular", "bool");
        add_td("num_ccs");
        add_td("nodes_largest_cc");
        add_td("treewidth");
        add_td("bipartite", "bool");
        add_td("planar", "bool");

        if (RUN_MODE) {
            const sol = ins.solution;

            if (sol.score) {
                const url = `/api/solutions/download?iid=${ins.iid}&solver=${SOLVER}&run=${RUN}`;
                let score_elem = create_td("", "num", "score");

                score_elem.innerHTML = sol.score + "&nbsp;";

                {
                    let score_a = document.createElement("a");
                    score_a.href = url;
                    score_a.innerText = "⬇️";
                    score_elem.appendChild(score_a);
                }

                let score_diff_elem = create_td(sol.score - ins.best_score, "num", "score_diff");

                score_elem.classList.add("run-mode-begin");
                score_elem.classList.add("run-mode-only");
                score_diff_elem.classList.add("run-mode-only");

                if (sol.score == ins.best_score) {
                    score_elem.classList.add("optimal");
                    score_diff_elem.classList.add("optimal");
                }
            } else {
                let elem = document.createElement("td");
                let code = sol.error_code;
                if (code.startsWith("Incomplete")) { code = "Incomplete"; }
                elem.innerText = code;
                elem.colSpan = 2;
                elem.classList.add("status");
                elem.classList.add("run-mode-begin");

                if (code == "Incomplete" || code == "Timeout") {
                    elem.classList.add("warning");
                } else {
                    elem.classList.add("error");
                }

                elem.classList.add("run-mode-only");
                row.appendChild(elem);
            }

            const time = sol.seconds_computed.toFixed(1) + "s";
            let runtime_elem = create_td(time, "num", "runtime");
            runtime_elem.classList.add("run-mode-only");
        }

        ins.tags.forEach(tid => {
            row.querySelector(".tags").appendChild(createTagElement(tags[tid]));
        });

        tableBody.appendChild(row);
    });

    let tableHead = document.querySelector("#instances thead");
    tableHead.querySelectorAll(".asc").forEach(th => th.classList.remove("asc"));
    tableHead.querySelectorAll(".desc").forEach(th => th.classList.remove("desc"));
    tableHead.querySelector('#header_' + filterOptions.order_by).classList.add(filterOptions.direction);
}

function populateMaxValues(max_values) {
    function format(num) {
        if (num < 1000) {
            return num;
        } else if (num < 1000000) {
            return Math.floor(num / 1000) + "K";
        } else {
            return Math.floor(num / 1000000) + "M";
        }
    }

    function populate(key, word) {
        const num = max_values[key];

        document.querySelector(`#constr_${key}_lb`).disabled = (num === null);
        document.querySelector(`#constr_${key}_ub`).disabled = (num === null);

        if (num === null) {
            return;
        }

        let limit = Math.pow(10, Math.floor(Math.log10(num)));
        for (let bx = 1; bx <= limit; bx *= 10) {
            for (const s of [1, 2, 5]) {
                const x = bx * s;
                if (x > num) {
                    break;
                }

                let option = document.createElement("option");
                option.value = x;
                let formatted_x = format(x);
                option.innerText = "at least " + word.replace("$", formatted_x);
                document.querySelector(`#constr_${key}_lb`).appendChild(option);
            }
        }

        limit = Math.pow(10, Math.ceil(Math.log10(num)));
        for (let bx = 1; bx <= limit; bx *= 10) {
            for (const s of [1, 2, 5]) {
                const x = bx * s;
                let option = document.createElement("option");
                option.value = x;
                let formatted_x = format(x);
                option.innerText = "at most " + word.replace("$", formatted_x);
                document.querySelector(`#constr_${key}_ub`).appendChild(option);

                if (x > num) {
                    return;
                }
            }
        }
    }

    populate("nodes", "$ nodes");
    populate("edges", "$ edges");
    populate("min_deg", "min degree of $");
    populate("max_deg", "max degree of $");
    populate("num_ccs", "$ connected comps.");
    populate("nodes_largest_cc", "$ nodes in largest cc");
    populate("best_score", "$ nodes in domset");

    if (RUN_MODE) {
        populate("score", "$ nodes in domset");
        populate("score_diff", "$ more than best known");
        populate("seconds_computed", "$ seconds");
    }
}

function updateBoolFilters(key, elem) {
    const value = document.querySelector(`#bool_constr_${key}`).value;
    console.log(key, value);

    switch (value) {
        case "none":
            filterOptions[key] = null;
        case "true":
        case "false":
            filterOptions[key] = (value == "true");
    }

    fetchData();
}

function updateRangeFilters(key, _elem) {
    let value = parseInt(document.querySelector("#constr_" + key).value);

    const id = key.slice(0, -3);
    const ublb = key.slice(-2);

    const alt_key = id + "_" + ((ublb == "ub") ? "lb" : "ub");

    for (let opt of document.querySelectorAll(`#constr_${alt_key} option`)) {
        if (opt.value == "none") {
            continue;
        }

        if (value == "none") {
            opt.disabled = false;
        } else {
            opt.disabled = (ublb == "lb" && parseInt(opt.value) < value) || (ublb == "ub" && parseInt(opt.value) > value);
        }
    }

    fetchData();
}

function addButton(inner, listener, addClass = null) {
    let button = document.createElement("a");
    button.classList.add("page-link");
    button.href = "#";


    if ((typeof inner) == "string") {
        button.innerText = inner;
    } else {
        button.appendChild(inner);
    }

    button.addEventListener("click", listener);

    let li = document.createElement("li");
    li.classList.add("page-list");
    if (addClass !== null) {
        li.classList.add(addClass);
    }
    li.appendChild(button);
    document.querySelector("#pagination").appendChild(li);

    return li;
}



function setupPagination(current, total) {
    document.querySelector("#pagination").innerHTML = "";

    addButton("First", () => {
        filterOptions.current_page = 1;
        fetchData();
    }, (current != 1) ? null : "disabled");


    addButton("Prev", () => {
        filterOptions.current_page = current > 1 ? (current - 1) : 1;
        fetchData();
    }, (current > 1) ? null : "disabled");


    let first = (current < 10) ? 1 : (current - 10);
    let last = (first + 21 > total) ? total : (first + 21);
    for (let i = first; i <= last; i++) {
        addButton(`${i}`, () => {
            filterOptions.current_page = i;
            fetchData();
        }, (current == i) ? "active" : null);
    }

    addButton("Next", () => {
        filterOptions.current_page = current < total ? (current + 1) : total;
        fetchData();
    }, (current < total) ? null : "disabled");

    addButton("Last", () => {
        filterOptions.current_page = total;
        fetchData();
    }, (current < total) ? null : "disabled");


}

fetchData(include_tags = true, include_max_values = true);

document.querySelector("#download_list").onclick = function () {
    let opts = buildFilter();

    const opts_list = Object.entries(opts).map(([key, value]) => `${key}=${encodeURIComponent(value)}`);
    const params = opts_list.join("&");

    window.location.href = `${apiBase}instances/list_download?${params}`;
};

document.querySelectorAll("#instances thead .sortable").forEach((th) => {
    const name = th.id.replace("header_", "");
    th.addEventListener("click", () => {
        if (filterOptions.order_by == name) {
            filterOptions.direction = (filterOptions.direction == "asc") ? "desc" : "asc";
        } else {
            filterOptions.order_by = name;
            filterOptions.direction = "asc";
        }

        fetchData();
    });
});

document.querySelectorAll(".form-control").forEach((e) => {
    if (e.id.startsWith("constr_")) {
        const key = e.id.replace("constr_", "");
        e.addEventListener('change', (elem) => { updateRangeFilters(key, elem); });
    } else if (e.id.startsWith("bool_constr_")) {
        const key = e.id.replace("bool_constr_", "");
        e.addEventListener('change', (elem) => { updateBoolFilters(key, elem); });
    }
});

document.querySelector("#tag").addEventListener("change", () => {
    fetchData();
});

document.querySelector("#reset_filters").addEventListener("click", () => {
    document.querySelectorAll(".form-control").forEach((e) => {
        if (e.id.startsWith("constr_") || e.id.startsWith("bool_constr_")) {
            e.value = "none";
        }
    });

    filterOptions = {
        direction: "asc",
        order_by: "id",
        current_page: 1,
        tag: null
    };

    fetchData();
});


function populateRunHeader(data) {
    const run = data.runs[0];
    const name = run.name ? run.name : `Run ${run.run_uuid}`;

    document.querySelector("#run-name").innerText = name;
    document.querySelector("#run-description").innerText = run.description;

    const total_num = run.num_optimal + run.num_suboptimal + run.num_infeasible + run.num_error + run.num_timeout + run.num_incomplete;
    const total_width = (run.num_scheduled ? run.num_scheduled : total_num);

    let stats_tbody = document.querySelector("#run-stats tbody");
    stats_tbody.innerHTML = "";

    function add_row(title, key, cls) {
        const num = run["num_" + key];
        const time = num ? ((run["seconds_computed_" + key] / num).toFixed(1) + "s") : "n/a";

        let row = document.createElement("tr");
        if (cls) { row.classList.add(cls); }

        let title_elem = document.createElement("td");
        title_elem.innerText = title;
        row.appendChild(title_elem);

        let num_elem = document.createElement("td");
        num_elem.classList.add("num");
        num_elem.innerText = num;
        row.appendChild(num_elem);

        let frac_elem = document.createElement("td");
        frac_elem.classList.add("num");
        frac_elem.innerText = (num / total_width * 100).toFixed(1) + "%";
        row.appendChild(frac_elem);

        let time_elem = document.createElement("td");
        time_elem.classList.add("num");
        time_elem.innerText = time;
        row.appendChild(time_elem);


        stats_tbody.appendChild(row);
    }

    add_row("Optimal instances", "optimal", "optimal");
    add_row("Suboptimal instances", "suboptimal", "");
    add_row("No/incomplete sol.", "incomplete", "warning");
    add_row("Timeout instances", "timeout", "warning");
    add_row("Infeasible instances", "infeasible", "error");
    add_row("Error instances", "error", "error");
}

// setup header
if (RUN_MODE) {
    fetch(apiSolverRunList).then(response => response.json()).then(populateRunHeader);
}

const tooltipTriggerList = document.querySelectorAll('[data-bs-toggle="tooltip"]')
const tooltipList = [...tooltipTriggerList].map(tooltipTriggerEl => new bootstrap.Tooltip(tooltipTriggerEl))

fetch(apiBase + "status")
    .then(response => response.json())
    .then(data => {
        console.log(data);
        document.querySelector("#site-stats").innerText =
            `${data.num_instances} instances, ${data.num_jobs} solver results, and ${data.num_unique_solutions} unique solutions`;
    });


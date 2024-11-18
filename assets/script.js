const apiBase = '/api/';
const apiTags = apiBase + 'tags';
const apiInstances = apiBase + 'instances';

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

function buildFilterList() {
    let list = [
        "sort_direction=" + filterOptions["direction"],
        "sort_by=" + filterOptions["order_by"]
    ];

    if (document.querySelector("#tag").value != "none") {
        list.push("tag=" + document.querySelector("#tag").value);
    }

    document.querySelectorAll(".form-control").forEach((e) => {
        if (e.id.startsWith("constr_")) {
            if (e.value == "none") { return; }
            const key = e.id.replace("constr_", "");
            list.push(`${key}=${e.value}`);
        } else if (e.id.startsWith("bool_constr_")) {
            const key = e.id.replace("bool_constr_", "");
            if (e.value == "none") { return; }
            list.push(`${key}=${e.value}`);
        }
    });


    return list;
}

function fetchData(include_tags = false, include_max_values = false) {
    let opts = buildFilterList();

    document.querySelector("#reset_filters").disabled = (opts.length == 2);

    opts.push("page=" + filterOptions["current_page"]);
    opts.push("limit=100");

    if (include_tags) {
        opts.push("include_tag_list=true");
    }

    if (include_max_values) {
        opts.push("include_max_values=true");
    }

    opts = opts.join("&");

    {
        const cls = "table-primary";
        document.querySelectorAll(`#instances th.${cls}`).forEach((e) => { e.classList.remove(cls) });
        document.querySelector(`#instances #header_${filterOptions.order_by}`).classList.add(cls);
    }

    fetch(`${apiInstances}?${opts}`)
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
    const tableBody = document.querySelector('#instances tbody');
    tableBody.innerHTML = '';
    instances.forEach(ins => {
        const row = document.createElement('tr');

        ins.regular = (ins.min_deg == ins.max_deg);
        score = ins.best_known_solution ? ins.best_known_solution : 'n/a';


        function add_td(key, fmt = "num", if_unknown = "?", order_by_key = null) {
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

            let elem = document.createElement("td");
            elem.classList.add(fmt);

            if (order_by_key === null) {
                order_by_key = key;
            }

            if (filterOptions.order_by == order_by_key) {
                elem.classList.add("table-primary");
            }

            elem.innerText = value;
            row.appendChild(elem);
        }

        add_td("iid", "num", null, "id");

        {
            let name_elem = document.createElement("td");
            name_elem.innerHTML = `<span class="name">${ins.name}</span><span class="tags"></span><p class="desc">${ins.description}</p>`;
            row.appendChild(name_elem);
        }


        add_td("nodes");
        add_td("edges");
        add_td("best_known_solution", "num", "❓", "score");
        add_td("min_deg");
        add_td("max_deg");
        add_td("regular", "bool");
        add_td("num_ccs");
        add_td("nodes_largest_cc");
        add_td("treewidth");
        add_td("bipartite", "bool");
        add_td("planar", "bool");

        {
            let action_elem = document.createElement("td");
            action_elem.classList.add("tool");
            action_elem.innerHTML = `<a href="${apiBase}instances/download/${ins.iid}" alt="Download">⬇️</a>`;
            row.appendChild(action_elem);
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
        let limit = Math.pow(10, Math.floor(Math.log10(num)));
        for (let bx = 1; bx <= limit; bx *= 10) {
            for (const s of [1, 2, 5]) {
                const x = bx * s;
                if (x == 1) { continue; }
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
                if (x == 1) { continue; }
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

    if (max_values.score !== null) {
        populate("score", "$ nodes in domset");
    } else {
        document.querySelector("#score_lb").disabled = true;
        document.querySelector("#score_ub").disabled = true;
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
    let opts = buildFilterList();
    window.location.href = `${apiBase}instance_list?${opts.join("&")}`;
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
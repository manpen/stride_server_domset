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

function sort_by(name) {
    if (filterOptions.order_by == name) {
        filterOptions.direction = (filterOptions.direction == "asc") ? "desc" : "asc";
    } else {
        filterOptions.order_by = name;
        filterOptions.direction = "asc";
    }

    fetchData();
}

function click_on_tag(tid) {
    console.log(tid);
    if (filterOptions.tag == tid) {
        // remove
        filterOptions.tag = null;
    } else {
        filterOptions.current_page = 1;
        filterOptions.tag = tid;
    }

    populateTags();
    fetchData();
}

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

    tag.onclick = function (e) { click_on_tag(data.tid); };

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

    for (const id of ["min_nodes", "max_nodes", "min_edges", "max_edges", "min_score", "max_score"]) {
        let x = document.querySelector("#" + id).value;
        if (x != "none") {
            list.push(`${id}=${x}`);
        }
    }

    return list;
}

function fetchData(include_tags = false, include_max_values = false) {
    let opts = buildFilterList();

    opts.push("page=" + filterOptions["current_page"]);
    opts.push("limit=100");

    if (include_tags) {
        opts.push("include_tag_list=true");
    }

    if (include_max_values) {
        opts.push("include_max_values=true");
    }

    opts = opts.join("&");

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
            download_btn.innerText = `List of ${data.total_matches} matches`;
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

        score = ins.best_known_solution ? ins.best_known_solution : 'n/a';

        row.innerHTML = `<td>${ins.iid}</td>
                    <td>
                        <span class="name">${ins.name}</span>
                        <span class="tags"></span>
                        <p class="desc">${ins.description}</p>
                    </td>
                    <td class="num">${ins.nodes}</td>
                    <td class="num">${ins.edges}</td>
                    <td class="num">${score}</td>
                    <td>
                        <a href="#" alt="Details">🔍</a>
                        <a href="${apiBase}instances/download/${ins.iid}" alt="Download">⬇️</a>
                    </td>`;

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
    console.log(max_values);

    function format(num) {
        if (num < 1000) {
            return num;
        } else if (num < 1000000) {
            return Math.floor(num / 1000) + "K";
        } else {
            return Math.floor(num / 1000000) + "M";
        }
    }

    function populate(key, word, num) {
        let limit = Math.pow(10, Math.floor(Math.log10(num)));
        for (let bx = 10; bx <= limit; bx *= 10) {
            for (const s of [1, 2, 5]) {
                const x = bx * s;
                if (x > num) {
                    break;
                }

                let option = document.createElement("option");
                option.value = x;
                let formatted_x = format(x);
                option.innerText = `at least ${formatted_x} ${word}`;
                document.querySelector("#min_" + key).appendChild(option);
            }
        }

        limit = Math.pow(10, Math.ceil(Math.log10(num)));
        for (let bx = 10; bx <= limit; bx *= 10) {
            for (const s of [1, 2, 5]) {
                const x = bx * s;
                let option = document.createElement("option");
                option.value = x;
                let formatted_x = format(x);
                option.innerText = `at most ${formatted_x} ${word}`;
                document.querySelector("#max_" + key).appendChild(option);

                if (x > num) {
                    return;
                }
            }
        }
    }

    populate("nodes", "nodes", max_values.max_nodes);
    populate("edges", "edges", max_values.max_edges);

    if (max_values.max_solution_score !== null) {
        populate("score", "nodes in domset", max_values.max_solution_score);
    } else {
        document.querySelector("#min_score").disabled = true;
        document.querySelector("#max_score").disabled = true;
    }

    for (const id of ["nodes", "edges", "score"]) {
        for (const minmax of ["min", "max"]) {
            let k = minmax + "_" + id;
            console.log(k);
            document.querySelector('#' + k).addEventListener("change", () => {
                updateFilters(k);
            });
        }
    }

    document.querySelector("#tag").addEventListener("change", () => {
        fetchData();
    });
}

function updateFilters(key) {
    let value = document.querySelector("#" + key).value;

    const mm = key.split("_")[0];
    const id = key.split("_")[1];
    const alt_key = ((mm == "min") ? "max" : "min") + "_" + id;

    for (let opt of document.querySelectorAll(`#${alt_key} option`)) {
        if (opt.value == "none") {
            continue;
        }

        if (value == "none") {
            opt.disabled = false;
        } else {
            opt.disabled = (mm == "min" && opt.value < value) || (mm == "max" && opt.value > value);
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
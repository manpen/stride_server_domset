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
    const tagsBody = document.querySelector('#tags');
    tagsBody.innerHTML = '';

    for (let tid in tags) {
        tagsBody.appendChild(createTagElement(tags[tid], true));
    }
}

function fetchData(include_tags = false) {
    let opts = "page=" + filterOptions["current_page"];
    opts += "&limit=100";
    opts += "&sort_direction=" + filterOptions["direction"];
    opts += "&sort_by=" + filterOptions["order_by"];

    if (include_tags) {
        opts += "&include_tag_list=true";
    }

    if (filterOptions.tag !== null) {
        opts += "&tag=" + filterOptions.tag;
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

            populateTable(data.results);
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

fetchData(include_tags = true);
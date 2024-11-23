function isValidUUID(uuid) {
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
    return uuidRegex.test(uuid);
}

const SOLVER = new URLSearchParams(window.location.search).get('solver');
const RUN = new URLSearchParams(window.location.search).get('run');

if (!isValidUUID(SOLVER)) {
    alert('Invalid solver_uuid');
    window.location.href = '/';
}

if (!isValidUUID(RUN)) {
    alert('Invalid run_uuid');
    window.location.href = '/';
}

const apiBase = '/api/';
const apiTags = apiBase + 'tags';
const apiInstances = apiBase + `instances?solver=${SOLVER}&run=${RUN}`;
const apiSolverRunList = apiBase + `solver_run/list?solver=${SOLVER}&run=${RUN}`;

function instanceFmt(value, row) {
    return `<a href="api/instances/download/${row.iid}">${value}</a>`;
}


function scoreFmt(value, row) {
    if (value == "Infeasible") {
        return "<span class='error'>Infeasible</span>";
    }

    const delta = row.raw.solution.score - row.raw.best_score;
    const cls = delta == 0 ? ' class="optimal"' : "";
    const url = `/api/solutions/download?iid=${row.iid}&solver=${SOLVER}&run=${RUN}`;
    return `<a href=${url}${cls}>${value}</a>`;
}

function digitsFmt(value, row) {
    value = value.toFixed(0);

    if (value.length <= 3) {
        return value;
    }

    while (value.length % 3) {
        value = " " + value;
    }

    let result = "";
    for (let i = 0; i < value.length; i += 3) {
        result += value.substr(i, 3) + "'";
    }

    return result.slice(0, -1);
}

function populateTable(instances) {
    let data = [];
    instances.forEach(ins => {
        row = {
            'raw': ins,
        };

        function add_td(key, fmt = "num", if_unknown = "?", order_by_key = null) {
            let value = ins[key];

            if (value === null || value === undefined) {
                value = if_unknown;
            } else {
                if (fmt == "num") {
                    //
                } else {
                    value = value ? "✅" : "❌";
                }
            }

            row[key] = value;
        }

        add_td("iid", "num", null, "id");
        add_td("name");
        add_td("nodes");
        add_td("edges");

        add_td("best_score", "num", "❓");


        // score
        {

            let score = ins.solution.score;
            if (score === null) {
                row["score"] = ins.solution.error_code;
                row["score_delta"] = "";
            } else {
                row["score"] = score;
                row["score_delta"] = score - ins.best_score;
            }
        }

        // runtime
        {
            let runtime = ins.solution.seconds_computed;
            row["time"] = runtime.toFixed(runtime > 100 ? 0 : 2) + "s";
        }

        data.push(row);
    });

    $("#instances").bootstrapTable('load', data)
}

// setup header
fetch(apiSolverRunList).then(response => response.json()).then(data => {
    const run = data.runs[0];
    const name = run.name ? run.name : `Run ${run.run_uuid}`;

    document.querySelector("#run-name").innerText = name;
    document.querySelector("#run-description").innerText = run.description;

    const total_num = run.num_optimal + run.num_suboptimal + run.num_infeasible + run.num_error + run.num_timeout;
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
    add_row("Timeout instances", "timeout", "warning");
    add_row("Infeasible instances", "infeasible", "error");
    add_row("Error instances", "error", "error");

});

function plotRuntime(instances) {
    let solved = { x: [], y: [], mode: 'markers', type: 'scatter', name: "Solved" };
    let unsolved = { x: [], y: [], mode: 'markers', type: 'scatter', name: "Unsolved" };

    instances.forEach(ins => {
        let runtime = ins.solution.seconds_computed;
        if (runtime === null) { return; }

        if (ins.solution.score !== null) {
            solved.x.push(ins.nodes);
            solved.y.push(runtime);
        } else {
            unsolved.x.push(ins.nodes);
            unsolved.y.push(runtime);
        }
    });

    const layout = {
        xaxis: {
            title: 'Nodes',
            type: 'log',
            autorange: true
        },
        yaxis: {
            title: 'Runtime (s)',
            type: 'log',
            autorange: true
        },
    };

    let elem = document.getElementById('runtime-plot');
    Plotly.newPlot(elem, [solved, unsolved], layout);
}


function plotScore(instances) {
    let solved = { x: [], y: [], text: [], mode: 'markers', type: 'scatter', name: "Solved" };

    instances.forEach(ins => {
        const score = ins.solution.score;
        if (score === null) { return; }

        solved.x.push(ins.nodes);
        solved.y.push(score - ins.best_score);
        solved.text.push(`Instance: ${ins.iid}<br>Nodes: ${ins.nodes}<br>Edges: ${ins.edges}<br>Score: ${score}<br>Best Score: ${ins.best_score}<br>Runtime: ${ins.solution.seconds_computed.toFixed(2)}s`);
    });

    const layout = {
        xaxis: {
            title: 'Nodes',
            type: 'log',
            autorange: true
        },
        yaxis: {
            title: 'Score - Best Score',
            autorange: true
        },
    };

    let elem = document.getElementById('runtime-plot');
    Plotly.newPlot(elem, [solved], layout);
}

function plot(instances) {
    if (document.querySelector("#plot-type").value == "score") {
        plotScore(instances);
    } else {
        plotRuntime(instances);
    }
}

let instances = [];
document.querySelector("#plot-type").addEventListener("change", () => {
    if (instances.length > 0) {
        plot(instances);
    }
});


document.querySelector("#breadcrumb-solver").href = "/runs.html?solver=" + SOLVER;
document.querySelector("#download-instances").href = `/api/instance_list?solver=${SOLVER}&run=${RUN}`;

fetch(apiInstances)
    .then(response => response.json())
    .then(data => {
        instances = data.results;
        populateTable(instances);
        plot(instances);
    });




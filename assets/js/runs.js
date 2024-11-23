const SOLVER = new URLSearchParams(window.location.search).get('solver');
function isValidUUID(uuid) {
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
    return uuidRegex.test(uuid);
}

if (!isValidUUID(SOLVER)) {
    alert('Invalid solver UUID');
    window.location.href = '/';
}

document.querySelector("h1").innerText = `Solver ${SOLVER}`;


const apiBase = '/api/';
const apiSolverRunList = apiBase + `solver_run/list?solver=${SOLVER}`;
const apiSolverRunAnnotate = apiBase + `solver_run/annotate?solver=${SOLVER}`;

function getRunUuid(elem) {
    while (elem != document) {
        if (elem.run_uuid) {
            return elem.run_uuid;
        }
        elem = elem.parentNode;
    }
}


function populateRuns(data) {
    const filterOptions = data.options;
    document.querySelector("#instances_of").innerHTML = "<option value='all'>all instances</option>";

    let tbody = document.getElementById("runs");
    tbody.innerHTML = "";

    for (run of data.runs) {
        let row = document.createElement("div");
        row.classList.add("row");
        row.classList.add("run");
        row.classList.add("m-3");

        if (run.hide) {
            row.classList.add("hidden");
        }

        if (run.run_uuid == filterOptions.instances_of) {
            row.classList.add("instances_of");
        }

        row.id = `run-${run.run_uuid}`;
        row.run_uuid = run.run_uuid;

        let info = document.createElement("div");
        info.classList.add("info");
        info.classList.add("col-6");
        info.classList.add("p-6");
        info.classList.add("mt-3");
        function add_content(el, cls, text) {
            let elem = document.createElement(el);
            elem.className = cls;
            elem.innerText = text;
            info.appendChild(elem);
            return elem;
        }

        const name = run.name ? run.name : run.run_uuid;

        // add entry for instance filter
        {
            let option = document.createElement("option");
            option.value = run.run_uuid;
            option.innerText = `only of '${name}'`;

            if (filterOptions.instances_of == run.run_uuid) {
                option.selected = true;
            }

            document.querySelector("#instances_of").appendChild(option);
        }

        let h4 = add_content("h4", "name", name);
        h4.addEventListener("click", (e) => {
            const run = getRunUuid(e.target);
            window.location.href = `/run.html?solver=${SOLVER}&run=${run}`;
        });

        let tools = add_content("div", "tools", "");
        {
            function add_tool(cls, text, handler) {
                let a = document.createElement("a");
                a.classList.add(cls);
                a.innerText = text;
                tools.appendChild(a);
                if (handler) {
                    a.addEventListener("click", handler);
                }
                return a;
            }

            add_tool("hide", run.hide ? "[unhide]" : "[hide]", (e) => {
                const run = getRunUuid(e.target);
                const run_elem = document.querySelector(`#run-${run}`);
                const is_hidden = run_elem.classList.contains("hidden");
                const new_value = !is_hidden;

                if (new_value) {
                    run_elem.classList.add("hidden");
                    e.target.innerText = "[unhide]";
                } else {
                    run_elem.classList.remove("hidden");
                    e.target.innerText = "[hide]";
                }

                fetch(`${apiSolverRunAnnotate}&run=${run}&hide=${new_value}`);
            });

            add_tool("edit_name", " [change name]", (e) => {
                const run = getRunUuid(e.target);
                const h4 = document.querySelector(`#run-${run} h4`);
                const old_value = h4.innerText;
                const new_value = prompt("Enter new name", old_value);

                if (new_value && new_value != old_value) {
                    fetch(`${apiSolverRunAnnotate}&run=${run}&name=${encodeURIComponent(new_value)}`);
                    h4.innerText = new_value;
                }
            });

            add_tool("edit_description", " [change description]", (e) => {
                const run = getRunUuid(e.target);
                let p = document.querySelector(`#run-${run} p.desc`);
                const old_value = p.innerHTML;
                const new_value = prompt("Enter new description", old_value);

                if (new_value && new_value != old_value) {
                    fetch(`${apiSolverRunAnnotate}&run=${run}&description=${encodeURIComponent(new_value)}`);
                    p.innerText = new_value;
                }
            });

            if (run.run_uuid == filterOptions.instances_of) {
                add_tool("instances_of", " [all instances]", (e) => {
                    document.getElementById("instances_of").value = "all";
                    fetchRuns();
                });
            } else {
                add_tool("instances_of", " [only these instance]", (e) => {
                    const run = getRunUuid(e.target);
                    document.getElementById("instances_of").value = run;
                    fetchRuns();
                });
            }

        }


        add_content("span", "created", run.created_at);
        add_content("p", "desc", run.description);

        row.appendChild(info);

        ////

        const total_num =
            run.num_optimal + run.num_suboptimal + run.num_infeasible + run.num_error + run.num_timeout;

        run.num_error + run.num_infeasible + run.num_valid + run.num_optimal;
        const total_width = (run.num_scheduled ? run.num_scheduled : total_num);

        let right_col = document.createElement("div");
        right_col.classList.add("col-6");
        right_col.classList.add("mt-3");


        let progress = document.createElement("div");
        progress.classList.add("progress-stacked");
        progress.classList.add("p-0");


        function add_block(num, cls, title) {
            if (num == 0) { return; }
            let progress_outer = document.createElement("div");
            progress_outer.className = "progress";
            progress_outer.role = "progressbar";
            progress_outer.style.width = `${num / total_width * 100}%`;


            let inner_bar = document.createElement("div");
            inner_bar.classList.add("progress-bar");
            for (c of cls) {
                inner_bar.classList.add(c);
            }
            inner_bar.innerHTML = title.replace("$", (num / total_width * 100).toFixed(1) + "%");

            progress_outer.appendChild(inner_bar);
            progress.appendChild(progress_outer);

        }

        add_block(run.num_optimal, ["bg-success"], "Opt:<br/>$");
        add_block(run.num_suboptimal, [], "Subopt:<br/> $");
        add_block(run.num_infeasible, ["bg-danger"], "Infeasible:<br/> $");
        add_block(run.num_timeout, ["bg-warning"], "Timeout:<br/> $");
        add_block(run.num_error, ["bg-danger"], "Error:<br/> $");
        add_block(total_width - total_num, ["bg-info", "rogress-bar-striped"], "", "Scheduled:<br/> $");

        right_col.appendChild(progress);

        let right_info = document.createElement("div");
        right_info.classList.add("info");

        {
            function add_field(title, key, cls) {
                const num = run["num_" + key];
                if (num == 0) { return; }

                const time = num ? ((run["seconds_computed_" + key] / num).toFixed(1) + "s") : "n/a";

                let span = document.createElement("span");
                span.classList.add("field");
                if (cls) span.classList.add(cls);
                span.innerHTML = ` ${title}:&nbsp;${num}&nbsp;(mean:&nbsp;${time})`;

                right_info.appendChild(span);
            }

            let elem = document.createElement("span");
            elem.classList.add("field");
            elem.innerText = `Total: ${total_num} runs`;
            right_info.appendChild(elem);

            add_field("Optimal", "optimal", "text-optimal");
            add_field("Suboptimal", "suboptimal", "text-suboptimal");
            add_field("Infeasible", "infeasible", "text-infeasible");
            add_field("Timeout", "timeout", "text-warning");
            add_field("Error", "error", "text-error");
        }

        right_col.appendChild(right_info);

        row.appendChild(right_col);
        tbody.appendChild(row);
    }


}

function fetchRuns() {
    opts = "";

    if (document.getElementById("include_hidden").checked) {
        opts += "&include_hidden=true";
    }

    const instances_of = document.getElementById("instances_of").value;
    if (instances_of != "all") {
        opts += `&instances_of=${instances_of}`;
    }

    fetch(`${apiSolverRunList}${opts}`)
        .then(response => response.json())
        .then(data => populateRuns(data));
}

fetchRuns();

document.getElementById("include_hidden").addEventListener("change", fetchRuns);
document.getElementById("instances_of").addEventListener("change", fetchRuns);
const IID = parseInt(new URLSearchParams(window.location.search).get('iid'));
if (isNaN(IID)) {
    alert('Invalid Instance Id');
    window.location.href = '/';
}


let SOLVER = new URLSearchParams(window.location.search).get('solver');
let RUN = new URLSearchParams(window.location.search).get('run');

function isValidUUID(uuid) {
    const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
    return uuidRegex.test(uuid);
}

if ((SOLVER !== null) && !isValidUUID(SOLVER)) {
    alert('Invalid solver_uuid');
    SOLVER = null;
}

const SOLVER_MODE = (SOLVER !== null);

if (SOLVER_MODE) {
    document.querySelector("body").classList.add("solver-mode");
}

const API_BASE = '/api/';

function parseDimacsToD3(text) {
    let num_nodes = null;
    let num_edges = null;
    let lines = text.split("\n");

    let edges = [];

    lines.forEach(line => {
        line = line.trim();
        if (line == "" || line.startsWith("c")) {
            return;
        }

        if (line.startsWith("p")) {
            const comps = line.split(" ");
            console.assert(num_nodes === null);
            console.assert(comps.length == 4);

            num_nodes = parseInt(comps[2]);
            num_edges = parseInt(comps[3]);
            console.assert(!isNaN(num_nodes) && !isNaN(num_edges));

            return;
        }

        const parts = line.split(" ");
        console.assert(parts.length == 2);

        const u = parseInt(parts[0]);
        const v = parseInt(parts[1]);
        console.assert(!isNaN(u) && !isNaN(v));
        console.assert(0 < u && u <= num_nodes);
        console.assert(0 < v && v <= num_nodes);
        console.assert(u != v);

        edges.push({
            source: u,
            target: v
        });
    });

    console.assert(edges.length == num_edges);

    let nodes = [];
    for (let i = 1; i <= num_nodes; ++i) {
        nodes.push({
            id: i
        });
    }

    return {
        nodes: nodes,
        links: edges
    };
}

function parseSolution(text) {
    let lines = text.split("\n");

    let numbers = [];

    lines.forEach(line => {
        line = line.trim();
        if (line == "" || line.startsWith("c")) {
            return;
        }

        const num = parseInt(line);
        console.assert(!isNaN(num));

        numbers.push(num);
    });

    let size = numbers[0];
    numbers.shift();

    console.assert(numbers.length == size);

    return numbers;
}


function ForceGraph({
    nodes, // an iterable of node objects (typically [{id}, …])
    links // an iterable of link objects (typically [{source, target}, …])
}, {
    nodeId = d => d.id, // given d in nodes, returns a unique identifier (string)
    nodeGroup, // given d in nodes, returns an (ordinal) value for color
    nodeGroups, // an array of ordinal values representing the node groups
    nodeTitle, // given d in nodes, a title string
    nodeRadius = 5, // node radius, in pixels
    nodeStrength,
    linkSource = ({ source }) => source, // given d in links, returns a node identifier string
    linkTarget = ({ target }) => target, // given d in links, returns a node identifier string
    linkStroke = "#999", // link stroke color
    linkStrokeWidth = 1.5, // given d in links, returns a stroke width in pixels
    linkStrength,
    colors = d3.schemeTableau10, // an array of color strings, for the node groups
    width = 640, // outer width, in pixels
    height = 400, // outer height, in pixels
    invalidation // when this promise resolves, stop the simulation
} = {}) {
    // Compute values.
    const N = d3.map(nodes, nodeId).map(intern);
    const R = typeof nodeRadius !== "function" ? null : d3.map(nodes, nodeRadius);
    const LS = d3.map(links, linkSource).map(intern);
    const LT = d3.map(links, linkTarget).map(intern);
    if (nodeTitle === undefined) nodeTitle = (_, i) => N[i];
    const T = nodeTitle == null ? null : d3.map(nodes, nodeTitle);
    const G = nodeGroup == null ? null : d3.map(nodes, nodeGroup).map(intern);
    const W = typeof linkStrokeWidth !== "function" ? null : d3.map(links, linkStrokeWidth);
    const L = typeof linkStroke !== "function" ? null : d3.map(links, linkStroke);


    // Replace the input nodes and links with mutable objects for the simulation.
    nodes = d3.map(nodes, (_, i) => ({ id: N[i], covered: 0 }));
    links = d3.map(links, (_, i) => ({ source: LS[i], target: LT[i] }));

    // Compute default domains.
    if (G && nodeGroups === undefined) nodeGroups = d3.sort(G);

    // Construct the scales.
    const color = nodeGroup == null ? null : d3.scaleOrdinal(nodeGroups, colors);

    // Construct the forces.

    const forceNode = d3.forceManyBody();
    const forceLink = d3.forceLink(links).id(({ index: i }) => N[i]);
    const collideForce = d3.forceCollide(20);

    const simulation = d3.forceSimulation(nodes)
        .force("link", forceLink)
        .force("charge", forceNode)
        .force("collide", collideForce)
        .force("center", d3.forceCenter())
        .on("tick", ticked);


    function update_strength() {
        const slider = document.querySelector("#link-force-slider");
        const strength = parseInt(slider.value) / parseInt(slider.max) * 1.5;
        simulation.force("link").strength((d) =>
            ((covered_data === null || covered_data[d.source.id].in_ds != covered_data[d.target.id].in_ds) ? 1 : 0.25) * strength
            * (0.5 / Math.sqrt(graph_adj[d.source.id].size) + 0.5 / Math.sqrt(graph_adj[d.target.id].size))
        );
        simulation.alpha(1).restart();
    }
    update_strength();

    document.querySelector("#link-force-slider").addEventListener("change", (e) => {
        update_strength();
    });


    const svg = d3.create("svg")
        .attr("width", width)
        .attr("height", height)
        .attr("viewBox", [-width / 2, -height / 2, width, height])
        .attr("style", "max-width: 100%; height: auto; height: intrinsic;")
        .call(d3.zoom() // Add zoom behavior
            .scaleExtent([0.5, 5]) // Zoom scale range
            .on("zoom", (event) => {
                graph_group.attr("transform", event.transform); // Apply zoom transform
            }));


    const legend = svg.append("g").classed("legend", true).attr("transform", `translate(${-width / 2.1},${-height / 2.2})`);

    function add_row(i, cls, text) {
        const dy = 40;
        const dx = 30;
        const row = legend.append("g").classed("node", true).classed(cls, true).attr("transform", `translate(0,${dy * i})`);
        row.append("circle");
        row.append("text").attr("x", dx).attr("y", 4).text(text);
    }

    add_row(0, "domset", "In DomSet");
    add_row(1, "uniquecovered", "Covered uniquely");
    add_row(2, "multicovered", "Covered multiple times");

    const graph_group = svg.append("g");

    const link = graph_group.selectAll(".link")
        .data(links)
        .enter()
        .append("line")
        .attr("class", "link");

    const node = graph_group.selectAll(".node")
        .data(nodes)
        .enter()
        .append("g")
        .attr("class", "node")
        .on("mouseover", function (event, d) {
            graph_group.selectAll(".link").classed("faded",
                (o) => !(o.source.id == d.id || o.target.id == d.id));
            graph_group.selectAll(".node").classed("faded",
                (o) => !graph_adj[o.id].has(d.id)
            );
        })
        .on("mouseout", function (event, d) {
            graph_group.selectAll(".link").classed("faded", false);
            graph_group.selectAll(".node").classed("faded", false);
        })
        .call(drag(simulation));

    const circle = node.append("circle");


    if (T) {
        node.append("text").attr("text-anchor", "middle").attr("dy", 4).text(({ index: i }) => T[i]);
    }

    node.append("title").text(
        (d) => `Node ${d.id}, Degree: ${graph_adj[d.id].size - 1}`
    );

    if (invalidation != null) invalidation.then(() => simulation.stop());

    function intern(value) {
        return value !== null && typeof value === "object" ? value.valueOf() : value;
    }

    function ticked() {
        link
            .attr("x1", d => d.source.x)
            .attr("y1", d => d.source.y)
            .attr("x2", d => d.target.x)
            .attr("y2", d => d.target.y);

        node
            .attr("transform", d => `translate(${d.x}, ${d.y})`);

        if (covered_data !== null) {


        }
    }

    function updateNodeEdgeTypes() {
        if (covered_data === null) {
            for (cls in ["domset", "uniquecovered", "multicovered", "unused"]) {
                node.classed(cls, false);
                link.classed(cls, false);
            }
        } else {
            node.classed("domset", d => covered_data[d.id].in_ds);
            node.classed("multicovered", d => !covered_data[d.id].in_ds && covered_data[d.id].covered_by.length > 1);
            node.classed("uniquecovered", d => !covered_data[d.id].in_ds && covered_data[d.id].covered_by.length == 1);

            link.classed("unused", d => !covered_data[d.source.id].in_ds && !covered_data[d.target.id].in_ds);
            link.classed("multicovered",
                d => (covered_data[d.source.id].in_ds != covered_data[d.target.id].in_ds) &&
                    ((covered_data[d.source.id].in_ds && covered_data[d.target.id].covered_by.length > 1)
                        || (covered_data[d.target.id].in_ds && covered_data[d.source.id].covered_by.length > 1)
                    ));

            link.classed("uniquecovered",
                d => (covered_data[d.source.id].in_ds != covered_data[d.target.id].in_ds) &&
                    ((covered_data[d.source.id].in_ds && covered_data[d.target.id].covered_by.length == 1)
                        || (covered_data[d.target.id].in_ds && covered_data[d.source.id].covered_by.length == 1)
                    ));
        }

    }
    updateNodeEdgeTypes();

    function drag(simulation) {
        function dragstarted(event) {
            if (!event.active) simulation.alphaTarget(0.3).restart();
            event.subject.fx = event.subject.x;
            event.subject.fy = event.subject.y;
        }

        function dragged(event) {
            event.subject.fx = event.x;
            event.subject.fy = event.y;
        }

        function dragended(event) {
            if (!event.active) simulation.alphaTarget(0);
            event.subject.fx = null;
            event.subject.fy = null;
        }

        return d3.drag()
            .on("start", dragstarted)
            .on("drag", dragged)
            .on("end", dragended);
    }

    let obj = Object.assign(svg.node());
    obj.simulation = simulation;
    obj.ticked = ticked;
    obj.forceNode = forceNode;
    obj.forceLink = forceLink;
    obj.updateStrength = update_strength;
    obj.graphGroup = graph_group;
    obj.updateNodeEdgeTypes = updateNodeEdgeTypes;

    return obj;
}

var graph_data = null;
var graph_adj = null;
var graph_d3 = null;
var domset_data = null;
var covered_data = null;
var graph_tooltips = null

function update_graph_data() {
    if (domset_data !== null) {
        update_domset();
    }

    graph_adj = [null];
    graph_adj = graph_adj.concat(graph_data.nodes.map(node => {
        return new Set([node.id])
    }));


    graph_data.links.forEach(link => {
        graph_adj[link.source].add(link.target);
        graph_adj[link.target].add(link.source);
    });


    let graph_elem = document.getElementById("graph-container");
    graph_elem.innerHTML = '';

    graph_d3 = ForceGraph(graph_data, {
        nodeId: d => d.id,
        nodeGroup: d => 1,
        nodeTitle: d => d.id,
        nodeRadius: 14,
        width: graph_elem.offsetWidth,
        height: graph_elem.offsetHeight,
    });

    graph_elem.appendChild(graph_d3);
}

function update_domset() {
    if (graph_data === null) {
        covered_data = null;
    } else {

    covered_data = {};

    graph_data.nodes.forEach(node => {
        covered_data[node.id] = { in_ds: false, covered_by: [] };
    });

    if (domset_data !== null) {
        domset_data.forEach(node => {
            covered_data[node].in_ds = true;
            covered_data[node].covered_by.push(node);
        });
    }

    graph_data.links.forEach(link => {
        if (covered_data[link.source].in_ds) {
            covered_data[link.target].covered_by.push(link.source);
        }

        if (covered_data[link.target].in_ds) {
            covered_data[link.source].covered_by.push(link.target);
        }
    });

        if (graph_d3 !== null) {
            graph_d3.graphGroup.selectAll(".node title").text(function (d) {
                let text = `Node ${d.id}, Degree: ${graph_adj[d.id].size - 1}`;
                if (covered_data[d.id].in_ds) {
                    text += `\nIn DomSet`;
                } else {
                    text += `\nCoverage ${covered_data[d.id].covered_by.length} by node(s) [${covered_data[d.id].covered_by.join(", ")}]`;
                }
                return text;
            });

            graph_d3.updateStrength();
        }
    }

    if (graph_d3 !== null) {
        graph_d3.updateNodeEdgeTypes();
    }

}

function fetch_graph() {
    document.getElementById("graph-loading").style.display = "block";
    fetch(`${API_BASE}instances/download/${IID}`)
        .then(response => response.text())
        .then((text) => {
            graph_data = parseDimacsToD3(text);
            update_graph_data();
            document.getElementById("graph-loading").style.display = "none";
        });
}


var solution_caches = {};
function fetch_solution(run) {
    if (solution_caches[run]) {
        domset_data = solution_caches[run];
        update_domset();
        document.getElementById("graph-container").classList.remove("loading");
        return;
    }
    fetch(`${API_BASE}solutions/download?iid=${IID}&solver=${SOLVER}&run=${run}`)
        .then(response => response.text())
        .then((text) => {
            domset_data = parseSolution(text);
            solution_caches[run] = domset_data;
            update_domset();
            document.getElementById("graph-container").style.opacity = 1;
        });
}

document.getElementById("solution-select").addEventListener("change", (e) => {
    document.getElementById("graph-container").classList.add("loading");
    if (e.target.value === "none") {
        domset_data = null;
        update_domset();
    } else {
        fetch_solution(e.target.value);
    }
});

fetch(`${API_BASE}instance_solutions?iid=${IID}` + (SOLVER_MODE ? `&solver=${SOLVER}` : ''))
    .then(response => response.json())
    .then((data) => {
        if (SOLVER_MODE) {
            const solutions = document.getElementById("solution-select");
            solutions.innerHTML = '<option value="none">Solution to visualize</option>';

            let last_solution = "none";
            data.solver_solutions.forEach(sol => {
                if (!sol.run || !sol.score) { return; }
                const option = document.createElement("option");
                option.value = sol.run;
                option.text = (sol.run_name !== undefined ? sol.run_name : sol.run) + `, score: ${sol.score}`;
                solutions.appendChild(option);
                last_solution = sol.run;
            });
            solutions.value = RUN ? RUN : last_solution;
            fetch_solution(last_solution);
        }
    });

fetch(`${API_BASE}instances/list`, {
    method: 'POST',
    headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json'
    },
    body: JSON.stringify({ iid: IID })
})
    .then(response => response.json())
    .then((data) => {
        console.assert(data.results.length == 1);
        const inst = data.results[0];

        document.getElementById("instance-name").innerText = "Instance: " + inst.name;
        document.getElementById("instance-description").innerText = inst.description;

        if (inst.nodes < 100) {
            document.getElementById("graph-vis").classList.add("show");
            fetch_graph();
        } else {
            document.getElementById("graph-loading").style.display = "none";
            document.getElementById("graph-size-warning").style.display = "block";
            document.querySelector("#graph-size-warning button").addEventListener("click", (e) => {
                document.getElementById("graph-size-warning").style.display = "none";
                document.getElementById("graph-vis").classList.add("show");
                fetch_graph();
            });
        }
    });

const tooltipTriggerList = document.querySelectorAll('[data-bs-toggle="tooltip"]')
const tooltipList = [...tooltipTriggerList].map(tooltipTriggerEl => new bootstrap.Tooltip(tooltipTriggerEl))

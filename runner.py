#!/usr/bin/env python3
import argparse
import requests
import sqlite3
from pathlib import Path
import subprocess
import time
import hashlib
import uuid

ENDPOINT = 'http://localhost:8000/'

DB_DIR_PATH = Path("./.runner/")
DB_CACHE_PATH = DB_DIR_PATH / "instances.db"
DB_RUNNER_PATH = DB_DIR_PATH / "runner.db"

VERBOSE = False

def abort(message):
    print(message)
    exit(1)

def db_open_path(path):
    path.parent.mkdir(parents=True, exist_ok=True)

    def dict_factory(cursor, row):
        d = {}
        for idx, col in enumerate(cursor.description):
            d[col[0]] = row[idx]
        return d

    con = sqlite3.connect(path)
    con.row_factory = dict_factory
    return con

def db_open_runner_db():
     return db_open_path(DB_RUNNER_PATH)

def db_open_cache_db():
    db = db_open_path(DB_CACHE_PATH)
    db.execute(r"""CREATE TABLE IF NOT EXISTS InstanceData ( did INT AUTO_INCREMENT PRIMARY KEY, hash CHAR(64) NOT NULL, data LONGBLOB);""")
    return db

def fetch_instance_data_from_cache(data_hash):
    with db_open_cache_db() as conn:
        cursor = conn.cursor()
        cursor.execute('SELECT data FROM InstanceData WHERE hash = ?', (data_hash,))
        row = cursor.fetchone()
        
        if row is None:
            return None
        
        return row["data"]
    
def download_instance_data(instance_id, data_hash):
    url = ENDPOINT + f'api/instances/download/{instance_id}'
    if VERBOSE: print(f'Downloading instance from {url}')
    try:
        req = requests.get(url)
        req.raise_for_status()
    except requests.exceptions.HTTPError as e:
        abort(f"Failed to download instance\nError: {e}")

    data = req.text
    assert "p ds" in data, "Instance data does not contain header 'p ds'"

    if VERBOSE: print(f'Caching instance')
    with db_open_cache_db() as conn:
        cursor = conn.cursor()
        cursor.execute('INSERT INTO InstanceData (hash, data) VALUES (?, ?)', (data_hash, data))

    return data
    

def load_instance(instance_id):
    with db_open_runner_db() as conn:
        cursor = conn.cursor()
        cursor.execute('SELECT * FROM instance WHERE iid = ?', (instance_id,))
        instance_record = cursor.fetchone()

        assert instance_record is not None, 'Instance not found in runner database'
        
    hash = instance_record["data_hash"]

    data = fetch_instance_data_from_cache(hash)
    if data is None:
        data = download_instance_data(instance_id, hash)

    instance_record["data"] = data
    return instance_record

class SolutionSyntaxError(Exception): pass
class SolutionInfeasbileError(Exception): pass


def read_solution(data):
    """Read solution from data in the PACE format:
    - first line is number of nodes k in the solution
    - following lines are node numbers
    There are k + 1 lines in total, plus optional comments starting with 'c'.
    Empty lines are ignored.
    """
    try:
        lines = (x.strip() for x in data.split('\n'))
        numbers = [int(x) for x in lines if x and not x.startswith('c')]
    except Exception as e:
        raise SolutionSyntaxError("Failed to parse solution", e)
    
    if not numbers:
        raise SolutionInfeasbileError("Read empty solution")
    
    card = len(numbers) - 1
    if card != numbers[0]:
        raise SolutionInfeasbileError(f"Solution is header (len={numbers[0]}) is inconsistent with number of lines ({card} + 1)")

    return numbers[1:]

def read_instance(data):
    """Read instance from data in the PACE format:
    The header line starts with 'p ds' followed by number of nodes n and edges m.
    Following lines are edges in the format 'u v'.
    There are m+1 lines in total, plus optional comments starting with 'c'.
    Empty lines are ignored.
    Each edge exists only in one direction.
    """
    num_nodes, num_edges, adjlist = None, None, None

    edges_seen = 0

    for line in data.split('\n'):
        line = line.strip()
        if line.startswith("p ds"):
            parts = line.replace("  ", " ").split()
            assert len(parts) == 4, "Invalid header" 
            
            nodes = int(parts[2])
            edges = int(parts[3])

            adjlist = [[] for _ in range(nodes + 1)]

        elif line.startswith("c"):
            continue

        elif not line:
            continue

        else:
            assert adjlist is not None, "Header not found"

            e = line.split()
            u = int(e[0]) 
            v = int(e[1]) 
            edges_seen += 1

            assert 0 < u <= nodes, f"Invalid node {u}"
            assert 0 < v <= nodes, f"Invalid node {v}"

            adjlist[u].append(v)
            adjlist[v].append(u)

    assert edges_seen == edges, "Number of edges in header does not match number of edges in data"

    return (nodes, adjlist)    


def verify_solution(graph_nodes, graph_adjlist, solution):
    """Verify solution for graph with graph_nodes and graph_adjlist.
    Returns true on success and raises SolutionInfeasbileError on failure."""
    if len(solution) > graph_nodes:
        raise SolutionInfeasbileError("Solution has more nodes than graph")
    
    if any(not 1 <= i <= graph_nodes for i in solution):
        raise SolutionInfeasbileError("Solution has invalid node")

    covered = set()
    for u in solution:
        covered.update(graph_adjlist[u])

    if len(covered) != graph_nodes:
        raise SolutionInfeasbileError("Solution does not cover nodes", sorted(set(range(1, graph_nodes + 1)) - covered))
    
    return True

def compute_hash_of_solution(solution):
    hasher = hashlib.sha256()
    for number in solution:
        hasher.update(number.to_bytes(4, byteorder='little', signed=False))
    return hasher.hexdigest()


def execute_solver(args, instance_data):
    if VERBOSE: print("Execute solver ...")
    cmd = [args.solver]

    data = instance_data["data"]
    process = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    process.stdin.write(data)
    process.stdin.close()

    start = time.time()
    
    kill_sent = False
    result = None
    timeout = False
    while True:
        elapsed = time.time() - start
        retcode = process.poll()

        if retcode is not None:
            break

        if elapsed > args.timeout and not kill_sent:
            process.kill()
            if VERBOSE: print("Send kill signal")
            kill_sent = True

        elif elapsed > args.timeout + args.grace:
            process.terminate()
            if VERBOSE: print("Send term signal and ignore output")
            timeout = True
            break
            
        time.sleep(0.1 + min(0.5, elapsed / 10))
    
    result = process.stdout.read() if not timeout else None
    return {"result": result, "elapsed": elapsed, "timeout": timeout}

def upload_solution(args, instance_id, solution, solver_result):
    url = ENDPOINT + f'api/solutions/new'

    # TODO: implement caching and dont upload if hash is already in database

    params = {
            "instance_id": instance_id,
            "run_uuid": args.run,
            "seconds_computed": solver_result["elapsed"],
            "solution": solution,
    }

    if args.solver_uuid is not None:
        params["solver_uuid"] = args.solver_uuid

    if VERBOSE: print(f'Uploading result {url}')
    try:
        req = requests.post(url, json=params)
        req.raise_for_status()
    except requests.exceptions.HTTPError as e:
        abort(f"Failed to upload result \nError: {e}")



def run_command(args):
    if VERBOSE: print('Running solver {} on instance {}'.format(args.solver, args.instance))
    
    instance = load_instance(args.instance)
    assert instance is not None and instance.get('data') is not None, 'Instance not found'

    graph_nodes, graph_adjlist = read_instance(instance["data"])

    solver_result = execute_solver(args, instance)

    if solver_result is not None:
        solution = read_solution(solver_result["result"])
        is_valid = verify_solution(graph_nodes, graph_adjlist, solution)
        
        if VERBOSE and is_valid:
            print("Solution is valid")

        upload_solution(args, instance["iid"], solution, solver_result)
        

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('-v', '--verbose', action='store_true', help='Verbose output')

    subparsers = parser.add_subparsers(dest='command')

    run_parser = subparsers.add_parser('run')
    run_parser.add_argument('-s', '--solver', required=True, help='Path to solver to execute')
    run_parser.add_argument('-i', '--instance', required=True, help='Instance to solve')
    
    run_parser.add_argument('-r', '--run', help='UUID of the run; random if not provided')
    run_parser.add_argument('-t', '--timeout', type=int, default=300, help='Timeout in seconds')
    run_parser.add_argument('-g', '--grace', type=int, default=5, help='Grace period in seconds')

    args = parser.parse_args()

    if args.run is None:
        args.run = uuid.uuid4().hex


    args.solver_uuid = "49442d06-9d29-11ef-8b4a-4f6690149c60"
    VERBOSE = args.verbose
    
    if args.command == 'run':
        run_command(args)

    else:
        parser.print_help()


if __name__ == '__main__':
    main()

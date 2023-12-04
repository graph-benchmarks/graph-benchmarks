import matplotlib.pyplot as plt
import numpy as np
from matplotlib.ticker import MaxNLocator

import os
from datetime import datetime

def generate_line_graph(data_rows, output_directory, select_log_id, use_dataset):
    """Generate the graph and save it in a png file. The graph has multiple
    algorithm lines with on the horizontal axis the number of workers and on the
    vertical axis the execution time.

    Args:
        data_rows (tuple): the db row for the logged performance of one benchmark result (id, algo, dataset, cpu, workers, mem_size, log_type, time).
        output_directory (string): the path to the output directory from root.
        select_log_id (string): the log id to use (not implemented).
        use_dataset (string): the dataset to use on each algorithm. Skip all others.
    """
    
    data_groups = {}
    algorithms = set()    
    for row in data_rows:
        log_id, algo, dataset, log_type, time, vertex, edge, workers = row
        
        # Use only the right dataset.
        if dataset != use_dataset:
            continue
        
        algorithms.add(algo)
        
        if algo not in data_groups:
            data_groups[algo] = {"workers": [], "time": []}
            
        data_groups[algo]["workers"].append(workers)
        data_groups[algo]["time"].append(time)

    fig, ax = plt.subplots()
    
    # List of markers for dynamically assigning markers per line (algo).
    markers = ["o", "s", "D", "^", "v", "p", "*", "h", "<", ">"]

    # Create a mapping of workers to uniform indices.
    unique_workers = sorted(set().union(*(data_groups[algo]["workers"] for algo in algorithms)))
    worker_to_index = {worker: i for i, worker in enumerate(unique_workers)}

    for i, algo in enumerate(algorithms):
        marker = markers[i % len(markers)]
        
        # Sort the data per worker count of the dataset for each algorithm.
        sorted_indices = np.argsort(data_groups[algo]["workers"])
        sorted_workers = [data_groups[algo]["workers"][idx] for idx in sorted_indices]
        sorted_times = [data_groups[algo]["time"][idx] for idx in sorted_indices]
        
        sorted_indices = [worker_to_index[worker] for worker in sorted_workers]
    
        
        ax.plot(
            sorted_indices,
            sorted_times,
            label=algo,
            marker=marker
        )

    # Customize the plot
    ax.set_xlabel("Workers")
    ax.set_ylabel("Time")
    ax.set_title("Execution Time per Algorithm for Various Worker Counts")
    ax.legend()
    
    # Set x-ticks on the horizontal axis to show only the actual worker counts with equal spacing.
    ax.set_xticks(range(len(unique_workers)))
    ax.set_xticklabels(unique_workers)

    # Writing the graph to a file.
    timestamp_str = datetime.now().strftime("%Y%m%d%H%M")
    output_filename = os.path.join(
        output_directory,
        f"result-lines-{select_log_id}-{timestamp_str}.png"
        )
    plt.savefig(output_filename)
    print(f"Graph saved to {output_filename}")

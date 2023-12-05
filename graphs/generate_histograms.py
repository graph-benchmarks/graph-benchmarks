import re
import matplotlib.pyplot as plt
import numpy as np


import os
from datetime import datetime

def generate_histograms(data_rows, output_directory, select_log_ids):
    """Generate the graph and save it in a png file. The graph has multiple
    algorithm histrograms with multiple bars for each dataset performance
    per algorithm.

    Args:
        data_rows (tuple): the db row for the logged performance of one benchmark result (id, algo, dataset, cpu, workers, mem_size, log_type, time).
        output_directory (string): the path to the output directory from root.
        select_log_id (string): the log id to use (not implemented).
    """
    data_groups = {}
    algorithms = set()
    datasets = set()
    
    # Get the log_ids to use.
    log_ids = set([int(n) for n in sum([l.split(',') for l in re.findall(r'[\d,]+[,\d]', select_log_ids)], []) if n.isdigit()])

    for row in data_rows:
        log_id, algo, dataset, log_type, time, vertex, edge, workers = row

        algorithms.add(algo)
        datasets.add(dataset)
        if algo not in data_groups:
            data_groups[algo] = {}
        if dataset not in data_groups[algo]:
            data_groups[algo][dataset] = []
        data_groups[algo][dataset].append(time)

    # Sort the algorithms and datasets for consistent order.
    algorithms = sorted(algorithms)
    datasets = sorted(datasets)

    # Create a figure and axis
    fig, ax = plt.subplots()

    bar_width = 0.2

    # Create a list of x positions for each group of bars.
    x = np.arange(len(algorithms))

    # Create a bar for each dataset within each algorithm.
    for i, dataset in enumerate(datasets):
        times = [data_groups[algo][dataset][0] if dataset in data_groups[algo] else 0 for algo in algorithms]
        ax.barh(x + i * bar_width, times, height=bar_width, label=dataset)

    # Plot design.
    ax.set_xlabel("Time")
    ax.set_ylabel("Algorithms")
    ax.set_title(
        "Execution Time for Datasets per Algorithm",
        fontweight="bold"
    )
    ax.set_yticks(x + bar_width * (len(datasets) - 1) / 2)
    ax.set_yticklabels(algorithms)
    ax.legend(title="Datasets")
    plt.xticks(rotation=45, ha="right")
    plt.tight_layout()

    # Writing the graph to a file.
    timestamp_str = datetime.now().strftime("%Y%m%d%H%M")
    output_filename = os.path.join(
        output_directory,
        f"result-bar-{timestamp_str}.png"
        )
    plt.savefig(output_filename)
    print(f"Graph saved to {output_filename}")
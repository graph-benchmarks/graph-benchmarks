import psycopg2
import os
import time
from datetime import datetime
import matplotlib.pyplot as plt
import numpy as np

# DB connection & credential variables.
# TODO: use env variables with updated docker-compose file.
db_host = "benchmarks_db" 
db_port = "5432" 
db_name = "postgres" 
db_user = "user" 
db_password = "password"

# The output path of the generated grap. The eventual file name depends on the
# provided log id.
output_directory = "/app/results/"

# For now this id is not used, since the associated table and schema isn't
# defined yet.
select_log_id = os.environ.get("SELECT_LOG_ID")
print(f"Selecting data of log id: {select_log_id}")

try:
    # Connect to the PostgreSQL database
    connection = psycopg2.connect(
        host=db_host,
        port=db_port,
        database=db_name,
        user=db_user,
        password=db_password
    )

    # Connect to the databse.
    cursor = connection.cursor()
    print("connected to db...")

    # Query the data from the database.
    query = "SELECT * FROM driver_logging"
    cursor.execute(query)
    
    # Fetch the results and iterate over them. Group first per logged algorithm
    # and then per dataset for the specific algorithm.
    data_groups = {}
    algorithms = set()
    datasets = set()
    rows = cursor.fetchall() 
    for row in rows:
        log_id, algo, dataset, cpu, workers, mem_size, log_type, time = row

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
    
    # Some white space between the multiple groups of algorithms.
    bar_width = 0.2

    # Create a list of x positions for each group of bars.
    x = np.arange(len(algorithms))

    # Create a bar for each dataset within each algorithm.
    for i, dataset in enumerate(datasets):
        times = [data_groups[algo][dataset][0] if dataset in data_groups[algo] else 0 for algo in algorithms]
        ax.bar(x + i * bar_width, times, width=bar_width, label=dataset)

    # Plot design.
    ax.set_xlabel("Algorithms")
    ax.set_ylabel("Time")
    ax.set_title(
        "Execution Time for Different Datasets per Algorithm", 
        fontweight="bold"
    )
    ax.set_xticks(x + bar_width * (len(datasets) - 1) / 2)
    ax.set_xticklabels(algorithms)
    ax.legend(title="Datasets")
    plt.xticks(rotation=45, ha="right")
    plt.tight_layout()
    
    # Writing the graph to a file.
    timestamp_str = datetime.now().strftime("%Y%m%d%H%M")
    output_filename = os.path.join(
        output_directory, 
        f"result-{select_log_id}-{timestamp_str}.png"
        )
    plt.savefig(output_filename)
    print(f"Graph saved to {output_filename}")
    

except psycopg2.Error as error:
    print("Error connecting to PostgreSQL:", error)
    
    # Common reason for the connection to fail is that the db isn't able
    # to receive new connections yet. 
    # Therefore sleep before restarted by Docker.
    print("Go to sleep before exitting...")
    time.sleep(5)
    
    exit()

finally:
    # Close the cursor and connection
    if cursor:
        cursor.close()
    if connection:
        connection.close()

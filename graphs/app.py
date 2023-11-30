import psycopg2
import os
import time

from generate_histograms import generate_histograms
from generate_line_graph import generate_line_graph

# DB connection & credential variables.
db_host = os.environ.get("POSTGRES_HOST")
db_port = os.environ.get("POSTGRES_PORT")
db_user = os.environ.get("POSTGRES_USER")
db_password = os.environ.get("POSTGRES_PASSWORD")
db_name = os.environ.get("POSTGRES_DB")

# The output path of the generated grap. The eventual file name depends on the
# provided log id.
output_directory = "/app/results/"

# For now this id is not used, since the associated table and schema isn't
# defined yet.
select_log_id = os.environ.get("SELECT_LOG_ID")
print(f"Selecting data of log id: {select_log_id}")

lines_dataset = os.environ.get("GENERATE_LINES_DATASET")

def main():
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
        rows = cursor.fetchall() 
        
        generate_histograms(rows, output_directory, select_log_id)
        generate_line_graph(rows, output_directory, select_log_id, lines_dataset)
        

    except psycopg2.Error as error:
        print("Error connecting to PostgreSQL:", error)
        
        # Common reason for the connection to fail is that the db isn't able
        # to receive new connections yet. 
        # Therefore sleep before restarted by Docker.
        print("Go to sleep before exitting...")
        time.sleep(2)
        
        exit()

    finally:
        # Close the cursor and connection
        if cursor:
            cursor.close()
        if connection:
            connection.close()


if __name__ == "__main__":
    main()

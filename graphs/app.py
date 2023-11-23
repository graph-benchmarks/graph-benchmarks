import psycopg2
import time

# DB connection & credential variables.
# TODO: use env variables with updated docker-compose file.
db_host = "benchmarks_db" 
db_port = "5432" 
db_name = "postgres" 
db_user = "user" 
db_password = "password"

try:
    # Connect to the PostgreSQL database
    connection = psycopg2.connect(
        host=db_host,
        port=db_port,
        database=db_name,
        user=db_user,
        password=db_password
    )

    # Create a cursor object to interact with the database
    cursor = connection.cursor()
    print("connected to db...")

    # Execute a SELECT query
    query = "SELECT * FROM driver_logging"  # Replace with your table name and query
    cursor.execute(query)
    
    # Fetch and print the results
    rows = cursor.fetchall() 
    for row in rows:
        print(row)

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

# syntax=docker/dockerfile:1
FROM python:3.11-slim

WORKDIR /app

COPY requirements.txt requirements.txt

COPY . .

# Install the python dependencies.
RUN apt-get update \
    && apt-get -y install libpq-dev gcc \
    && pip install -r requirements.txt

# Run the application.
CMD ["python", "./app.py"] 
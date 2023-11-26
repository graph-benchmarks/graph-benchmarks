FROM python:3.11-slim
WORKDIR /app
COPY ..
RUN apt update && apt install build-essential -y
ADD gs_driver.py requirements.txt
RUN python3 -m pip install --upgrade pip
RUN python3 - m pip install -r requirements.txt
ENTRYPOINT ["python", "gs_driver.py"]

FROM python:3.11-slim
WORKDIR /app
RUN apt update && apt install gcc -y
ADD ./requirements.txt /app/requirements.txt
RUN pip install -r requirements.txt
ADD . /app
ENTRYPOINT ["python", "huge_driver.py"]

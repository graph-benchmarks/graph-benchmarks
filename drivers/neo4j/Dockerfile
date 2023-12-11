FROM python:3.11-slim
WORKDIR /app
RUN apt update && apt install gcc curl -y
RUN curl -fsSL -o get_helm.sh https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3
RUN chmod 700 get_helm.sh
RUN ./get_helm.sh
ADD ./requirements.txt /app/requirements.txt
RUN pip install -r requirements.txt
ADD . /app
ENTRYPOINT ["python", "neo_driver.py"]

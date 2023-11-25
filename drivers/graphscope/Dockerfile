FROM python:3.11-slim
ADD gs_driver.py requirements.txt
RUN python3 -m pip install --upgrade pip
RUN python3 - m pip install -r requirements.txt
CMD ["python", "gs_driver.py"]

FROM node:21.4
WORKDIR /app
ADD . /app
RUN echo "installing node_modules"
RUN npm install
RUN echo "installing arangosh"
RUN apt install curl
RUN curl -OL https://download.arangodb.com/arangodb311/DEBIAN/Release.key
RUN apt-key add - < Release.key
RUN echo 'deb https://download.arangodb.com/arangodb311/DEBIAN/ /' | tee /etc/apt/sources.list.d/arangodb.list
RUN apt-get install apt-transport-https
RUN apt-get update && apt-get install arangodb3=3.11.6-1
ENTRYPOINT ["node", "arango_driver.js"]

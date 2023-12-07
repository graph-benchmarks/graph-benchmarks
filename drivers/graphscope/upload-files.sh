#!/bin/bash
hadoop --config /scratch fs -rm -r /attached
hadoop --config /scratch fs -mkdir /attached
hadoop --config /scratch fs -put $1 $1
hadoop --config /scratch fs -put $2 $2
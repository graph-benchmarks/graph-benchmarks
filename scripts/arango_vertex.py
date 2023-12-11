import sys

input_file = sys.argv[1] # only for vertex files
output_file = sys.argv[2]

wf = open(output_file, "w+")
wf.write("vertex\n")

with open(input_file, "r") as rf:
    while True:
        ln = rf.readline()
        if not ln:
            break
        wf.write(ln)

wf.close()
    

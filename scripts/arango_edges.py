import sys

input_file = sys.argv[1]    # only for edge files
output_file = sys.argv[2]
weights = bool(int(sys.argv[3]))     # 0 or 1

wf = open(output_file, "w+")

if weights:
    wf.write('src,dst,weights\n')
else:
    wf.write('src,dst\n')

with open(input_file, "r") as rf:
    while True:
        ln = rf.readline()
        if not ln:
            break
        props = ln.split(" ") 
        for i in range(len(props)):
            props[i] = "vertex/" + props[i]
        ln = str.join(",", props)
        wf.write(ln)

wf.close()

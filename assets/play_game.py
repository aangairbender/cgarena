import sys, subprocess, json, tempfile, os, re

if __name__ == '__main__':
    f, log_file = tempfile.mkstemp(prefix='log_')
    os.close(f)

    n_players = len(sys.argv) - 2
    seed = sys.argv[1]
    
    # assumes brutaltester-compatible referee.jar is placed in the same folder
    cmd = 'java --add-opens java.base/java.lang=ALL-UNNAMED -jar referee.jar' + ''.join([f' -p{i} "{sys.argv[i + 1]}"' for i in range(1, n_players+1)]) + f' -seed {seed} -l "{log_file}"'
    task = subprocess.run(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    with open(log_file, 'r') as f:
        json_log = json.load(f)
    os.remove(log_file)
    p_scores = []
    try:
        p_scores = [int(json_log['scores'][str(i)]) for i in range(n_players)]
    except:
        print(json_log['failCause'], file=sys.stderr)
        exit(1)
    rv = {}
    rv['ranks'] = [sum([int(p_score < p2_score) for p2_score in p_scores]) for p_score in p_scores] # assumes higher score is better
    rv['errors'] = [int(p_score < 0) for p_score in p_scores] # assumes negative score means error
    rv['attributes'] = []

    pattern = r"\[(T|P)DATA\](?:\[(\d+)\])?\s+(\w+)\s*=\s*(.+)"
    regex = re.compile(pattern, re.IGNORECASE)

    for player, key in enumerate([str(i) for i in range(n_players)]):
        for data in json_log['errors'][key]:
            if not data: continue
            for line in [line.strip() for line in data.split('\n')]:
                match = regex.match(line)
                if not match: continue

                type_tag = match.group(1).upper()  # T or P
                turn = match.group(2)             # optional number
                key = match.group(3)
                value = match.group(4)

                attribute = {
                    'name': key,
                    'player': player if type_tag == 'P' else None,
                    'turn': int(turn) if turn else None,
                    'value': value,
                }
                
                rv['attributes'].append(attribute)
                
    print(json.dumps(rv))
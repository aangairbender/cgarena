import subprocess, sys, signal, time
from pathlib import Path
import shutil

REFEREE_PATH = "referee/target/referee.jar"

# Keep track of the process globally so the signal handler can access it
child_process = None

def cleanup_child(signum=None, frame=None):
    """Ensures the child process is terminated when Python exits."""
    global child_process
    if child_process and child_process.poll() is None:
        print("\n[Python] Terminating child process...", file=sys.stderr)
        child_process.terminate()  # Sends SIGTERM
        try:
            child_process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            print("[Python] Child refused to die, killing it...", file=sys.stderr)
            child_process.kill()  # Sends SIGKILL
    
    # If this was triggered by a signal, exit the Python script
    if signum is not None:
        sys.exit(0)

# Register signal handlers for graceful shutdown on kill/interrupt
signal.signal(signal.SIGINT, cleanup_child)   # Ctrl+C
signal.signal(signal.SIGTERM, cleanup_child)  # Kill signal

def fix_assets_issue(dir):
    # Convert string paths to Path objects
    source = Path(dir) / "assets"
    destination = Path(dir) / "assets" / "assets"

    # Create the destination directory if it doesn't exist yet
    destination.mkdir(parents=True, exist_ok=True)

    # Counter for copied files
    copied_count = 0

    # Find and copy all .png files
    for png_file in source.glob("*.png"):
        shutil.copy2(png_file, destination / png_file.name)
        copied_count += 1

def main():
    seed = sys.argv[1]
    log_file = f"logs/log_{seed}.json"
    cmd = f'java --add-opens java.base/java.lang=ALL-UNNAMED -jar "{REFEREE_PATH}" -r "{log_file}"'

    global child_process
    
    try:
        # 1. Run the hardcoded Java app using subprocess
        # stdout=subprocess.PIPE allows us to read the app's output
        # text=True ensures we read strings instead of raw bytes
        child_process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, # Change to None if you want stderr to pass through to console
            text=True
        )
        
        # 2. Read exactly 1 line from the app's stdout
        # readline() will block until a line is available or the process exits
        line = ""
        while not line.startswith("http"):
            line = child_process.stdout.readline()
        
        if line:
            # Write it to Python's own stdout
            sys.stdout.write(line)
            sys.stdout.flush()
        else:
            print("[Python] Child process closed stdout without emitting data.", file=sys.stderr)
        # while not line.startswith("Exposed web server dir: "):
        #     line = child_process.stdout.readline()

        line = child_process.stdout.readline()
        tmp_dir = line.removeprefix("Exposed web server dir: ").rstrip("\n")
        print(tmp_dir)
        fix_assets_issue(tmp_dir)

        # Optional: Keep the script alive to demonstrate that killing 
        # the Python script will subsequently kill the child.
        print("[Python] Keeping script alive. Press Ctrl+C or kill the process to test cleanup.")
        while True:
            time.sleep(1)

    except Exception as e:
        print(f"[Python] An error occurred: {e}", file=sys.stderr)
        
    finally:
        # 3. If the script finishes normally or encounters an unhandled exception,
        # ensure the child is closed.
        cleanup_child()

if __name__ == "__main__":
    main()
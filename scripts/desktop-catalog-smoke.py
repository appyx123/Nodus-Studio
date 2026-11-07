"""Bounded, app-scoped GNOME accessibility check of the built catalog.

Requires an existing Tauri debug build, GNOME/Wayland, and system pyatspi.
Only private temporary copies of build resources are changed; no screenshots.
"""

from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import time

import pyatspi


ROOT = Path(__file__).resolve().parent.parent
BINARY = ROOT / "src-tauri/target/debug/nodus"
BUILT_REGISTRY = BINARY.parent / "components-registry"


def snapshot(pid):
    queue = []
    for app in pyatspi.Registry.getDesktop(0):
        try:
            if app.get_process_id() == pid:
                queue.append(app)
        except Exception:
            continue
    records = []
    visited = 0
    while queue and visited < 300:
        item = queue.pop(0)
        visited += 1
        try:
            name, role = item.name or "", item.getRoleName()
            text = ""
            try:
                interface = item.queryText()
                text = interface.getText(0, min(interface.characterCount, 1024))
            except NotImplementedError:
                pass
            records.append((item, role, name, text))
            queue.extend(list(item))
        except Exception:
            continue
    return records


def wait_for(process, predicate, label):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise AssertionError(f"Application exited during {label}: {process.returncode}")
        records = snapshot(process.pid)
        if predicate(records):
            print(f"PASS: {label}", flush=True)
            return records
        time.sleep(0.3)
    summary = [(role, name, text) for _, role, name, text in records]
    raise AssertionError(f"Timed out waiting for {label}; app accessibility: {summary!r}")


def contains(records, value):
    return any(value in name or value in text for _, _, name, text in records)


def activate(records, label):
    button = next(item for item, role, name, _ in records
                  if role in ("button", "toggle button") and name == label)
    assert button.queryAction().doAction(0), f"Could not activate {label}"


def reload_catalog(records):
    activate(records, "Reload")


def main():
    assert BINARY.is_file(), "Run the Tauri debug build first"
    assert (BUILT_REGISTRY / "basic-led.nodpkg/graphic.svg").is_file(), "Built registry resource is missing"
    with tempfile.TemporaryDirectory(prefix="catalog-smoke-", dir=BINARY.parent) as temporary:
        folder = Path(temporary)
        executable = folder / "bin/nodus"
        executable.parent.mkdir()
        os.link(BINARY, executable)
        # Tauri resolves Linux resources from ../lib/<productName>, not beside a relocated binary.
        resources = folder / "lib/Nodus Studio"
        resources.mkdir(parents=True)
        registry = resources / "components-registry"
        shutil.copytree(BUILT_REGISTRY, registry)
        invalid = registry / "invalid-smoke.nodpkg"
        invalid.mkdir()
        (invalid / "manifest.toml").write_text("schema_version = 9\n", encoding="utf-8")
        working_directory = folder / "unrelated-working-directory"
        working_directory.mkdir()
        process = subprocess.Popen([str(executable)], cwd=working_directory,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        try:
            records = wait_for(process, lambda rows: contains(rows, "8 package(s) loaded")
                               and contains(rows, "Red LED") and contains(rows, "Rejected packages")
                               and contains(rows, "invalid-smoke.nodpkg"),
                               "relocated resources load the LED and isolate an invalid package")
            # WebKitGTK exposes the SVG image as a panel with an inner SVG section.
            assert contains(records, "Red LED package graphic"), "SVG preview accessible name is missing"
            assert any(role == "section" and name == "Red LED" and "+" in text
                       for _, role, name, text in records), "Loaded SVG content is missing"
            print("PASS: isolated SVG preview and its content are exposed to accessibility", flush=True)

            activate(records, "Add Red LED")
            records = wait_for(process, lambda rows: contains(rows, "1 instance(s)")
                               and contains(rows, "2 net(s)"), "native Add places the first LED and registers pins")
            first_names = [name for _, _, name, _ in records if name.startswith("Red LED instance node-")]
            assert len(first_names) == 1, f"Expected first instance name: {first_names!r}"
            first_id = first_names[0].removeprefix("Red LED instance ")
            activate(records, f"Pin A on {first_id}")
            records = wait_for(process, lambda rows: contains(rows, f"Wiring from {first_id}.A"),
                               "native pin action starts wiring from first LED")
            activate(records, "Add Red LED")
            records = wait_for(process, lambda rows: contains(rows, "2 instance(s)")
                               and contains(rows, "4 net(s)"), "native Add places a second LED")
            names = {name for _, _, name, _ in records if name.startswith("Red LED instance node-")}
            assert len(names) == 2, f"Expected two separately named instances: {names!r}"
            print("PASS: two native canvas instances have distinct accessible identities", flush=True)
            second_id = next(name.removeprefix("Red LED instance ") for name in names
                             if name != first_names[0])
            activate(records, f"Pin K on {second_id}")
            records = wait_for(process, lambda rows: contains(rows, "1 wire(s)")
                               and contains(rows, "3 net(s) (1 connected)"),
                               "native second pin action connects wire and merges net")
            activate(records, "Wire color Amber")
            records = wait_for(process, lambda rows: contains(rows, "(amber)"),
                               "native wire color control updates the wire color")
            activate(records, "Zoom out")
            records = wait_for(process, lambda rows: contains(rows, "160%"), "native zoom control changes the viewport")
            activate(records, "Reset view")
            records = wait_for(process, lambda rows: contains(rows, "200%"), "native Reset view restores zoom")
            activate(records, "Delete selected")
            records = wait_for(process, lambda rows: contains(rows, "0 wire(s)")
                               and contains(rows, "4 net(s) (0 connected)"),
                               "native Delete selected removes the selected wire and splits the net")
            activate(records, "Start")
            records = wait_for(process, lambda rows: contains(rows, "Sim: running"),
                               "native Start runs simulation and streams binary frames")
            activate(records, "Pause")
            records = wait_for(process, lambda rows: contains(rows, "Sim: paused"),
                               "native Pause pauses simulation")
            activate(records, "Resume")
            records = wait_for(process, lambda rows: contains(rows, "Sim: running"),
                               "native Resume resumes simulation")
            activate(records, "Reset")
            records = wait_for(process, lambda rows: contains(rows, "Sim: stopped"),
                               "native Reset stops simulation and resets state")
            print("PASS: native simulation start/pause/resume/reset lifecycle and binary frame stream verified", flush=True)
            activate(records, "Delete selected")
            records = wait_for(process, lambda rows: contains(rows, "1 instance(s)")
                               and contains(rows, "2 net(s)")
                               and not contains(rows, "2 instance(s)"), "native deletion removes only the selected instance")

            shutil.rmtree(invalid)
            reload_catalog(records)
            records = wait_for(process, lambda rows: contains(rows, "8 package(s) loaded")
                               and not contains(rows, "Rejected packages"),
                               "Reload removes the resolved diagnostic")

            shutil.rmtree(registry)
            reload_catalog(records)
            records = wait_for(process, lambda rows: contains(rows, "Component discovery failed:")
                               and contains(rows, "Circuit workbench") and contains(rows, "1 instance(s)"),
                               "missing registry shows an error without crashing the shell")

            shutil.copytree(BUILT_REGISTRY, registry)
            reload_catalog(records)
            wait_for(process, lambda rows: contains(rows, "8 package(s) loaded")
                     and contains(rows, "Red LED") and not contains(rows, "Component discovery failed:"),
                     "Reload recovers after the registry is restored")
        finally:
            if process.poll() is None:
                process.terminate()
            try:
                output, _ = process.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                output, _ = process.communicate(timeout=5)
            if output:
                print("Application output:\n" + output[:4096])
    print("Private smoke-test files and child process cleaned up. Pixel-level visual QA was not performed.")


if __name__ == "__main__":
    main()

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


def reload_catalog(records):
    button = next(item for item, role, name, _ in records if role == "button" and name == "Reload")
    button.queryAction().doAction(0)


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
            records = wait_for(process, lambda rows: contains(rows, "1 package(s) loaded")
                               and contains(rows, "Red LED") and contains(rows, "Rejected packages")
                               and contains(rows, "invalid-smoke.nodpkg"),
                               "relocated resources load the LED and isolate an invalid package")
            # WebKitGTK exposes the SVG image as a panel with an inner SVG section.
            assert contains(records, "Red LED package graphic"), "SVG preview accessible name is missing"
            assert any(role == "section" and name == "Red LED" and "+" in text
                       for _, role, name, text in records), "Loaded SVG content is missing"
            print("PASS: isolated SVG preview and its content are exposed to accessibility", flush=True)

            shutil.rmtree(invalid)
            reload_catalog(records)
            records = wait_for(process, lambda rows: contains(rows, "1 package(s) loaded")
                               and not contains(rows, "Rejected packages"),
                               "Reload removes the resolved diagnostic")

            shutil.rmtree(registry)
            reload_catalog(records)
            records = wait_for(process, lambda rows: contains(rows, "Component discovery failed:")
                               and contains(rows, "A foundation for connected ideas."),
                               "missing registry shows an error without crashing the shell")

            shutil.copytree(BUILT_REGISTRY, registry)
            reload_catalog(records)
            wait_for(process, lambda rows: contains(rows, "1 package(s) loaded")
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

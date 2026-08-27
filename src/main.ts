import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface PhoneDevice {
  id: string;
  name: string;
  address: string;
  needs_pairing: boolean;
}

const deviceList = document.getElementById("device-list")!;
const deviceUl = deviceList.querySelector("ul")!;
const emptyState = document.getElementById("empty-state")!;
const statusEl = document.getElementById("status")!;

let devices: PhoneDevice[] = [];
let streaming = false;

function setStatus(msg: string, isError = false): void {
  statusEl.textContent = msg;
  statusEl.classList.toggle("error", isError);
}

async function connect(device: PhoneDevice): Promise<void> {
  setStatus(`Connecting to ${device.name}…`);
  await invoke("connect_device", { deviceId: device.id });
  streaming = true;
  setStatus(`Streaming ${device.name}.`);
}

function renderDevices(): void {
  deviceList.hidden = devices.length === 0;
  emptyState.hidden = devices.length > 0;
  deviceUl.replaceChildren(
    ...devices.map((d) => {
      const li = document.createElement("li");

      const label = document.createElement("span");
      label.className = "device-name";
      label.textContent = d.needs_pairing ? `${d.name} (needs pairing)` : d.name;
      li.appendChild(label);

      if (d.needs_pairing) {
        const code = document.createElement("input");
        code.type = "text";
        code.inputMode = "numeric";
        code.maxLength = 6;
        code.placeholder = "Pairing code";
        code.className = "pair-code";

        const pairBtn = document.createElement("button");
        pairBtn.textContent = "Pair";
        pairBtn.addEventListener("click", () => {
          void (async () => {
            if (!/^\d{6}$/.test(code.value)) {
              setStatus("Enter the 6-digit code shown on your phone.", true);
              return;
            }
            pairBtn.disabled = true;
            setStatus(`Pairing with ${d.name}…`);
            try {
              await invoke("pair_device", { address: d.address, code: code.value });
              setStatus(`Paired with ${d.name}. Connecting…`);
              // The device now advertises _adb-tls-connect; its saved address
              // is still valid, so connect immediately.
              await connect(d);
              renderDevices();
            } catch (e) {
              setStatus(`Pairing failed: ${String(e)}`, true);
              pairBtn.disabled = false;
            }
          })();
        });

        li.appendChild(code);
        li.appendChild(pairBtn);
      } else {
        const btn = document.createElement("button");
        btn.textContent = streaming ? "Disconnect" : "Connect";
        btn.addEventListener("click", () => {
          void (async () => {
            btn.disabled = true;
            try {
              if (streaming) {
                await invoke("disconnect_stream");
                streaming = false;
                setStatus("Disconnected.");
              } else {
                await connect(d);
              }
              renderDevices();
            } catch (e) {
              setStatus(`Connection failed: ${String(e)}`, true);
              btn.disabled = false;
            }
          })();
        });
        li.appendChild(btn);
      }

      return li;
    }),
  );
}

async function init(): Promise<void> {
  await listen<PhoneDevice>("phone-found", (e) => {
    devices = [...devices.filter((d) => d.id !== e.payload.id), e.payload];
    renderDevices();
  });

  await listen<string>("phone-lost", (e) => {
    devices = devices.filter((d) => d.id !== e.payload);
    renderDevices();
  });

  document.getElementById("btn-walkthrough")?.addEventListener("click", () => {
    void invoke("open_walkthrough");
  });

  document.getElementById("btn-troubleshoot")?.addEventListener("click", () => {
    void (async () => {
      setStatus("Running health checks…");
      try {
        const findings = await invoke<string[]>("run_health_check");
        setStatus(findings.length > 0 ? findings.join(" ") : "All checks passed.");
      } catch (e) {
        setStatus(`Health check failed: ${String(e)}`, true);
      }
    })();
  });

  setStatus("Searching for phones on this network…");
  try {
    await invoke("start_discovery");
  } catch (e) {
    setStatus(`Discovery failed to start: ${String(e)}`, true);
  }
}

void init();
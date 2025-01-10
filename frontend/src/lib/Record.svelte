<script>
  import { onMount } from "svelte";

  let isRecording = $state(false);

  onMount(() => {
    fetch("http://localhost:8080/api/state", {
      method: "GET",
    })
      .then((response) => {
        return response.text();
      })
      .then((body) => {
        isRecording = body == "Capture";
      });
  });

  async function record() {
    isRecording = !isRecording;
    let response = await fetch("http://localhost:8080/api/control", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        record: isRecording,
      }),
    });

    let body = await response.text();

    if (body == "Capture") {
      isRecording = true;
    }
    if (body == "Livefeed") {
      isRecording = false;
    }
  }
</script>

<button onclick={record}> {!isRecording ? "Record" : "Stop"} </button>

<style>
  button {
    position: absolute;
    bottom: 32px;
    right: 32px;
    border-radius: 100%;
    aspect-ratio: 1;
  }
</style>

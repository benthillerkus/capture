<script>
  let x = $state(0);
  let y = $state(0);

  $effect(() => {
    fetch("http://localhost:8080/api/control", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ convergence: { x, y } }),
    });
  });
</script>

<div>
  <label>
    X
    <input bind:value={x} type="number" />
  </label>
  <label>
    Y
    <input bind:value={y} type="number" />
  </label>
</div>

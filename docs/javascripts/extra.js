document$.subscribe(function() {
  // manual -> cli

  createPlayer('merged.cast', 'demo', {  loop: true,
                    poster: 'npt:0:1',
                    cols: 100,
                    rows:32,
                    mkap_theme: "none",
                    markers: [
                        [3.0, "Introduction"],
                        [42.0, "Launching the GUI"],
                        [46.0, "Demo: Launching the GUI"],
                        [56.0, "Regular cd"],
                        [65.0, "Demo: Regular cd"],
                        [80.0, "Going back to a recorded directory"],
                        [92.0, "Demo: Going back to a recorded directory"],
                        [106, "Filtering"],
                        [120, "Demo: Filtering"],
                        [140, "Conclusion"]
                    ]
                });

});

function createPlayer(src, containerId, opts, setup) {
  const container = document.getElementById(containerId);
  opts = {  ...opts };

  if (container !== null) {
    document.fonts.load("1em Fira Mono").then(() => {
      const player = AsciinemaPlayer.create(src, container, {
        terminalFontFamily: "'Fira Mono', monospace",
        ...opts
      });

      if (typeof setup === 'function') {
        setup(player);
      }
    }).catch(error => {
      const player = AsciinemaPlayer.create(src, container, opts);

      if (typeof setup === 'function') {
        setup(player);
      }
    });
  }
}

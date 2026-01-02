document$.subscribe(function() {
  // manual -> cli

  createPlayer('merged.cast', 'demo', {  loop: true,
                    poster: 'npt:0:1',
                    cols: 80,
                    rows: 24,
                    mkap_theme: "none",
                    markers: [
                        [2.0, "Introduction"],
                        [28.0, "Launching the GUI"],
                        [44.0, "Regular cd"],
                        [65.0, "Going back to a recorded directory"],
                        [88, "Filtering"],
                        [113, "Shortcuts"],
                        [175, "Conclusion"]
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

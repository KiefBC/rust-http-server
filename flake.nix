{
  description = "Development environment for the Rust HTTP server";

  inputs.nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0";

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      forEachSupportedSystem =
        function:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          function {
            inherit system;
            pkgs = import nixpkgs { inherit system; };
          }
        );
    in
    {
      devShells = forEachSupportedSystem (
        { pkgs, system }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              curl
              rust-analyzer
              rustc
              rustfmt
              self.formatter.${system}
            ];

            RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
          };
        }
      );

      formatter = forEachSupportedSystem ({ pkgs, ... }: pkgs.nixfmt);
    };
}

{
	description = "Moonlight GUI Library";

	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
	};

	outputs = { self, nixpkgs, ... }:
		let
			allSystems = [ "x86_64-linux" ];
			forAllSystems = f: nixpkgs.lib.genAttrs allSystems (system: f {
    				pkgs = import nixpkgs { inherit system; };
			});
		in
		{
    			packages = forAllSystems ({ pkgs }: {
        			default =
        			let
        				buildInputs = with pkgs; [
	        				wayland
	        				libxkbcommon
	        				libGL
        				];
        			in
					pkgs.rustPlatform.buildRustPackage {
						name = "moonlight";
						version = "0.1.0";
						src = self;
						inherit buildInputs;
						nativeBuildInputs = with pkgs; [
							makeWrapper
							pkg-config
						];
						cargoHash = "";
						dontPatchELF = true;
					};
    			});
    			devShells = forAllSystems ({ pkgs }: {
	    			default =
					let
						buildInputs = with pkgs; [
							cargo
							libxkbcommon
							libGL
							vulkan-headers
							vulkan-loader
							wayland
						];
					in
						pkgs.mkShell {
							inherit buildInputs;
				    			LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
				    			RUST_LOG = "info";
				    			shellHook = "echo Success";
			    			};
    			});
		};
}

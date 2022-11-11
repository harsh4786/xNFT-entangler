import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { XNftEntangler } from "../target/types/x_nft_entangler";

describe("xNFT-entangler", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.XNftEntangler as Program<XNftEntangler>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});

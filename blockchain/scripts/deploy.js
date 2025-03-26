const hre = require("hardhat");

async function main() {
  console.log("Deploying MyNFT contract...");
  
  // Get the contract factory
  const MyNFT = await hre.ethers.getContractFactory("MyNFT");
  
  // Deploy the contract
  const myNFT = await MyNFT.deploy();
  await myNFT.waitForDeployment();
  
  const address = await myNFT.getAddress();
  console.log("MyNFT deployed to:", address);
}

// Execute the deployment
main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
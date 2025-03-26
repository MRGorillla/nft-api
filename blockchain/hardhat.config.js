require("@nomicfoundation/hardhat-toolbox");
require("dotenv").config();

/** @type import('hardhat/config').HardhatUserConfig */
module.exports = {
  solidity: "0.8.28",
  networks: {
    ganache: {
      url: "http://127.0.0.1:7545",
      chainId: 1337, // Add this line - Ganache default chain ID
      accounts: [
        "0xb4d59920ba76441bbfcf9e6f517528cb75dcf7542aa454b966f0aa85724383be",
        "0xdec19af6050e59c92ee005a26751b2172e58c6814eb5f6f4c6c53dffefd17686"
      ]
    }
  }
};
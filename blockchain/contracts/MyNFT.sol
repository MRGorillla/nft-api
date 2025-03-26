// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract MyNFT is ERC721URIStorage, Ownable {
    uint256 private _nextTokenId;
    
    constructor() ERC721("TechhacksNFT", "TNFT") {
        // Ownable constructor is called automatically and sets msg.sender as owner
    }
    
    function mintNFT(address recipient, string memory tokenURI) 
        public 
        returns (uint256) 
    {
        uint256 tokenId = _nextTokenId++;
        
        _mint(recipient, tokenId);
        _setTokenURI(tokenId, tokenURI);
        
        return tokenId;
    }
}
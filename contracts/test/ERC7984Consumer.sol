// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC7984, IERC7984Receiver} from "../interfaces/IERC7984.sol";

/**
 * @title ERC7984Consumer
 * @dev Example contract that consumes ERC-7984 tokens.
 *
 * This contract demonstrates that any contract expecting the standard
 * ERC-7984 interface can work with our ERC7984ConfidentialToken wrapper.
 * It serves as a proof of interface compatibility.
 */
contract ERC7984Consumer is IERC7984Receiver {
    /// @dev The ERC-7984 token this consumer works with
    IERC7984 public token;

    /// @dev Tracks received transfers for testing
    struct ReceivedTransfer {
        address from;
        bytes32 amount;
        bytes data;
        uint256 timestamp;
    }

    /// @dev Array of all received transfers
    ReceivedTransfer[] public receivedTransfers;

    /// @dev Event emitted when a transfer is received
    event TransferReceived(address indexed from, bytes32 indexed amount);

    /// @dev Event emitted when balance is queried
    event BalanceQueried(address indexed account, bytes32 balance);

    constructor(IERC7984 _token) {
        token = _token;
    }

    /**
     * @dev Demonstrates querying token metadata via ERC-7984 interface.
     */
    function queryMetadata()
        external
        view
        returns (string memory tokenName, string memory tokenSymbol, uint8 tokenDecimals)
    {
        tokenName = token.name();
        tokenSymbol = token.symbol();
        tokenDecimals = token.decimals();
    }

    /**
     * @dev Demonstrates querying confidential balance via ERC-7984 interface.
     */
    function queryBalance(address account) external returns (bytes32 balance) {
        balance = token.confidentialBalanceOf(account);
        emit BalanceQueried(account, balance);
    }

    /**
     * @dev Demonstrates querying confidential total supply via ERC-7984 interface.
     */
    function queryTotalSupply() external view returns (bytes32) {
        return token.confidentialTotalSupply();
    }

    /**
     * @dev Demonstrates setting an operator via ERC-7984 interface.
     */
    function approveOperator(address operator, uint48 until) external {
        token.setOperator(operator, until);
    }

    /**
     * @dev Demonstrates checking operator status via ERC-7984 interface.
     */
    function checkOperator(address holder, address operator) external view returns (bool) {
        return token.isOperator(holder, operator);
    }

    /**
     * @dev Demonstrates initiating a transfer via ERC-7984 interface.
     * @param to Recipient address
     * @param amount The amount commitment
     * @param data The encoded (encryptedAmount, proof)
     */
    function initiateTransfer(
        address to,
        bytes32 amount,
        bytes calldata data
    ) external returns (bytes32) {
        return token.confidentialTransfer(to, amount, data);
    }

    /**
     * @dev Demonstrates initiating a transferFrom via ERC-7984 interface.
     * @param from Sender address (caller must be operator)
     * @param to Recipient address
     * @param amount The amount commitment
     * @param data The encoded (encryptedAmount, proof)
     */
    function initiateTransferFrom(
        address from,
        address to,
        bytes32 amount,
        bytes calldata data
    ) external returns (bytes32) {
        return token.confidentialTransferFrom(from, to, amount, data);
    }

    /**
     * @dev ERC-7984 receiver callback.
     * Called when tokens are transferred to this contract.
     */
    function onConfidentialTokenReceived(
        address from,
        bytes32 amount,
        bytes calldata data
    ) external override returns (bytes4) {
        // Record the transfer
        receivedTransfers.push(ReceivedTransfer({
            from: from,
            amount: amount,
            data: data,
            timestamp: block.timestamp
        }));

        emit TransferReceived(from, amount);

        // Return the magic value to accept the transfer
        return IERC7984Receiver.onConfidentialTokenReceived.selector;
    }

    /**
     * @dev Returns the number of transfers received.
     */
    function getReceivedTransferCount() external view returns (uint256) {
        return receivedTransfers.length;
    }

    /**
     * @dev Returns details of a received transfer.
     */
    function getReceivedTransfer(uint256 index)
        external
        view
        returns (address from, bytes32 amount, bytes memory data, uint256 timestamp)
    {
        ReceivedTransfer storage t = receivedTransfers[index];
        return (t.from, t.amount, t.data, t.timestamp);
    }
}

/**
 * @title ERC7984RejectingReceiver
 * @dev A receiver that rejects all transfers, for testing error handling.
 */
contract ERC7984RejectingReceiver is IERC7984Receiver {
    function onConfidentialTokenReceived(
        address,
        bytes32,
        bytes calldata
    ) external pure override returns (bytes4) {
        // Return wrong magic value to reject
        return bytes4(0xdeadbeef);
    }
}

/**
 * @title ERC7984RevertingReceiver
 * @dev A receiver that reverts, for testing error handling.
 */
contract ERC7984RevertingReceiver is IERC7984Receiver {
    error AlwaysReverts();

    function onConfidentialTokenReceived(
        address,
        bytes32,
        bytes calldata
    ) external pure override returns (bytes4) {
        revert AlwaysReverts();
    }
}

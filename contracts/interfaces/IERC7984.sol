// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title IERC7984
 * @dev Interface for the ERC-7984 Confidential Fungible Token standard.
 *
 * This interface defines a standard for confidential fungible tokens where
 * balances and transfer amounts are represented as encrypted pointers (bytes32).
 * The resolution and manipulation of these pointers is implementation specific.
 *
 * See https://eips.ethereum.org/EIPS/eip-7984
 */
interface IERC7984 {
    /**
     * @dev Emitted when a confidential transfer occurs.
     * @param from The sender address (zero address for mints)
     * @param to The recipient address (zero address for burns)
     * @param amount The encrypted amount pointer
     */
    event ConfidentialTransfer(
        address indexed from,
        address indexed to,
        bytes32 indexed amount
    );

    /**
     * @dev Emitted when an operator is set or unset.
     * @param holder The token holder who set the operator
     * @param operator The operator address
     * @param until The timestamp until which the operator is valid (0 to revoke)
     */
    event OperatorSet(
        address indexed holder,
        address indexed operator,
        uint48 until
    );

    /**
     * @dev Emitted when an encrypted amount is disclosed.
     * @param handle The encrypted amount pointer that was disclosed
     * @param amount The plaintext amount that was disclosed
     */
    event AmountDisclosed(bytes32 indexed handle, uint256 amount);

    /**
     * @dev Returns the name of the token.
     */
    function name() external view returns (string memory);

    /**
     * @dev Returns the symbol of the token.
     */
    function symbol() external view returns (string memory);

    /**
     * @dev Returns the number of decimals used for display purposes.
     */
    function decimals() external view returns (uint8);

    /**
     * @dev Returns the encrypted total supply of tokens.
     * @return The encrypted total supply as a bytes32 pointer
     */
    function confidentialTotalSupply() external view returns (bytes32);

    /**
     * @dev Returns the encrypted balance of an account.
     * @param account The address to query
     * @return The encrypted balance as a bytes32 pointer
     */
    function confidentialBalanceOf(address account) external view returns (bytes32);

    /**
     * @dev Returns whether an operator is authorized for a holder.
     * @param holder The token holder
     * @param spender The potential operator
     * @return True if the spender is an authorized operator
     */
    function isOperator(address holder, address spender) external view returns (bool);

    /**
     * @dev Authorizes an operator to transfer tokens on behalf of the caller.
     * @param operator The address to authorize
     * @param until The timestamp until which the authorization is valid
     */
    function setOperator(address operator, uint48 until) external;

    /**
     * @dev Transfers tokens confidentially.
     * @param to The recipient address
     * @param amount The encrypted amount pointer
     * @return The actual transferred amount pointer
     */
    function confidentialTransfer(address to, bytes32 amount) external returns (bytes32);

    /**
     * @dev Transfers tokens confidentially with additional data.
     * @param to The recipient address
     * @param amount The encrypted amount pointer
     * @param data Additional data (e.g., ZK proof)
     * @return The actual transferred amount pointer
     */
    function confidentialTransfer(
        address to,
        bytes32 amount,
        bytes calldata data
    ) external returns (bytes32);

    /**
     * @dev Transfers tokens on behalf of another account.
     * @param from The sender address
     * @param to The recipient address
     * @param amount The encrypted amount pointer
     * @return The actual transferred amount pointer
     */
    function confidentialTransferFrom(
        address from,
        address to,
        bytes32 amount
    ) external returns (bytes32);

    /**
     * @dev Transfers tokens on behalf of another account with additional data.
     * @param from The sender address
     * @param to The recipient address
     * @param amount The encrypted amount pointer
     * @param data Additional data (e.g., ZK proof)
     * @return The actual transferred amount pointer
     */
    function confidentialTransferFrom(
        address from,
        address to,
        bytes32 amount,
        bytes calldata data
    ) external returns (bytes32);
}

/**
 * @title IERC7984Receiver
 * @dev Interface for contracts that want to receive ERC-7984 token transfers.
 *
 * Contracts implementing this interface can be notified when they receive
 * confidential tokens, similar to ERC-721's IERC721Receiver pattern.
 */
interface IERC7984Receiver {
    /**
     * @dev Called when tokens are transferred to this contract.
     *
     * Implementers MUST return the function selector `onConfidentialTokenReceived.selector`
     * (bytes4(keccak256("onConfidentialTokenReceived(address,bytes32,bytes)"))) = 0x25483763
     * to indicate successful receipt and acceptance of the transfer.
     *
     * Returning any other value or reverting will cause the transfer to be rejected.
     *
     * @param from The sender address
     * @param amount The encrypted amount pointer
     * @param data Additional data passed with the transfer
     * @return The magic value `0x25483763` (this function's selector) to accept the transfer
     */
    function onConfidentialTokenReceived(
        address from,
        bytes32 amount,
        bytes calldata data
    ) external returns (bytes4);
}

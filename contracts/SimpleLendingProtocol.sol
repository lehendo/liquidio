// SPDX-License-Identifier: MIT
pragma solidity ^0.8.21;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

/**
 * @title SimpleLendingProtocol
 * @dev A simplified lending protocol for liquidation bot testing
 * 
 * Key Features:
 * - Deposit ETH as collateral
 * - Borrow stablecoin against collateral
 * - Health factor based liquidation (collateral / debt ratio)
 * - Liquidation bonus for liquidators
 */
contract SimpleLendingProtocol {
    IERC20 public immutable stablecoin;
    
    // Liquidation parameters
    uint256 public constant LIQUIDATION_THRESHOLD = 150; // 150% collateralization required
    uint256 public constant LIQUIDATION_BONUS = 110; // 10% bonus for liquidators
    uint256 public constant PRECISION = 100;
    
    // ETH price in stablecoin (for simplicity, hardcoded at $2000)
    // In production, this would use an oracle
    uint256 public ethPriceUSD = 2000 * 1e18;
    
    // User positions
    struct Position {
        uint256 collateral; // ETH deposited (in wei)
        uint256 debt; // Stablecoin borrowed
    }
    
    mapping(address => Position) public positions;
    
    // Events for monitoring
    event Deposit(address indexed user, uint256 amount);
    event Withdraw(address indexed user, uint256 amount);
    event Borrow(address indexed user, uint256 amount);
    event Repay(address indexed user, uint256 amount);
    event Liquidate(
        address indexed liquidator,
        address indexed user,
        uint256 debtRepaid,
        uint256 collateralSeized
    );
    
    constructor(address _stablecoin) {
        stablecoin = IERC20(_stablecoin);
    }
    
    /**
     * @dev Deposit ETH as collateral
     */
    function deposit() external payable {
        require(msg.value > 0, "Must deposit some ETH");
        positions[msg.sender].collateral += msg.value;
        emit Deposit(msg.sender, msg.value);
    }
    
    /**
     * @dev Withdraw collateral (if health factor allows)
     */
    function withdraw(uint256 amount) external {
        Position storage pos = positions[msg.sender];
        require(pos.collateral >= amount, "Insufficient collateral");
        
        pos.collateral -= amount;
        
        // Check health factor after withdrawal
        if (pos.debt > 0) {
            require(getHealthFactor(msg.sender) >= PRECISION, "Would become undercollateralized");
        }
        
        (bool success, ) = msg.sender.call{value: amount}("");
        require(success, "ETH transfer failed");
        
        emit Withdraw(msg.sender, amount);
    }
    
    /**
     * @dev Borrow stablecoin against collateral
     */
    function borrow(uint256 amount) external {
        Position storage pos = positions[msg.sender];
        require(pos.collateral > 0, "No collateral deposited");
        
        pos.debt += amount;
        
        // Check health factor after borrow
        require(getHealthFactor(msg.sender) >= PRECISION, "Would become undercollateralized");
        
        require(stablecoin.transfer(msg.sender, amount), "Transfer failed");
        
        emit Borrow(msg.sender, amount);
    }
    
    /**
     * @dev Repay borrowed stablecoin
     */
    function repay(uint256 amount) external {
        Position storage pos = positions[msg.sender];
        require(pos.debt >= amount, "Repaying more than debt");
        
        pos.debt -= amount;
        
        require(stablecoin.transferFrom(msg.sender, address(this), amount), "Transfer failed");
        
        emit Repay(msg.sender, amount);
    }
    
    /**
     * @dev Liquidate an undercollateralized position
     * @param user The address of the user to liquidate
     * @param debtToCover Amount of debt to repay
     */
    function liquidate(address user, uint256 debtToCover) external {
        Position storage pos = positions[user];
        require(pos.debt > 0, "No debt to liquidate");
        require(getHealthFactor(user) < PRECISION, "Position is healthy");
        require(debtToCover > 0 && debtToCover <= pos.debt, "Invalid debt amount");
        
        // Calculate collateral to seize (with liquidation bonus)
        uint256 collateralValue = (debtToCover * 1e18) / ethPriceUSD;
        uint256 collateralToSeize = (collateralValue * LIQUIDATION_BONUS) / PRECISION;
        
        require(collateralToSeize <= pos.collateral, "Not enough collateral");
        
        // Update position
        pos.debt -= debtToCover;
        pos.collateral -= collateralToSeize;
        
        // Transfer stablecoin from liquidator
        require(stablecoin.transferFrom(msg.sender, address(this), debtToCover), "Transfer failed");
        
        // Transfer collateral to liquidator
        (bool success, ) = msg.sender.call{value: collateralToSeize}("");
        require(success, "ETH transfer failed");
        
        emit Liquidate(msg.sender, user, debtToCover, collateralToSeize);
    }
    
    /**
     * @dev Calculate health factor of a position
     * Returns value scaled by PRECISION (100 = 100%)
     * Health factor = (collateral value * threshold) / debt
     */
    function getHealthFactor(address user) public view returns (uint256) {
        Position memory pos = positions[user];
        
        if (pos.debt == 0) {
            return type(uint256).max; // No debt = infinite health
        }
        
        uint256 collateralValueUSD = (pos.collateral * ethPriceUSD) / 1e18;
        uint256 maxBorrow = (collateralValueUSD * PRECISION) / LIQUIDATION_THRESHOLD;
        
        return (maxBorrow * PRECISION) / pos.debt;
    }
    
    /**
     * @dev Check if a position can be liquidated
     */
    function isLiquidatable(address user) external view returns (bool) {
        return positions[user].debt > 0 && getHealthFactor(user) < PRECISION;
    }
    
    /**
     * @dev Get position details
     */
    function getPosition(address user) external view returns (uint256 collateral, uint256 debt, uint256 healthFactor) {
        Position memory pos = positions[user];
        return (pos.collateral, pos.debt, getHealthFactor(user));
    }
    
    /**
     * @dev Update ETH price (for testing - simulates price oracle)
     */
    function setEthPrice(uint256 newPrice) external {
        ethPriceUSD = newPrice;
    }
    
    // Allow contract to receive ETH
    receive() external payable {}
}



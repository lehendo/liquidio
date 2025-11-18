// SPDX-License-Identifier: MIT
pragma solidity ^0.8.21;

import "forge-std/Test.sol";
import "../SimpleLendingProtocol.sol";
import "../MockERC20.sol";

contract LendingProtocolTest is Test {
    SimpleLendingProtocol public protocol;
    MockERC20 public stablecoin;
    
    address public user1 = address(0x1);
    address public user2 = address(0x2);
    address public liquidator = address(0x3);
    
    function setUp() public {
        // Deploy stablecoin with 1M supply
        stablecoin = new MockERC20("USD Stablecoin", "USDC", 1_000_000 * 1e18);
        
        // Deploy lending protocol
        protocol = new SimpleLendingProtocol(address(stablecoin));
        
        // Fund protocol with stablecoin for lending
        stablecoin.transfer(address(protocol), 500_000 * 1e18);
        
        // Fund users with ETH
        vm.deal(user1, 100 ether);
        vm.deal(user2, 100 ether);
        vm.deal(liquidator, 100 ether);
        
        // Fund liquidator with stablecoin
        stablecoin.transfer(liquidator, 100_000 * 1e18);
    }
    
    function testDeposit() public {
        vm.prank(user1);
        protocol.deposit{value: 10 ether}();
        
        (uint256 collateral, uint256 debt, ) = protocol.getPosition(user1);
        assertEq(collateral, 10 ether);
        assertEq(debt, 0);
    }
    
    function testBorrow() public {
        // Deposit 10 ETH as collateral
        vm.prank(user1);
        protocol.deposit{value: 10 ether}();
        
        // Borrow $10,000 (50% of collateral value at $2000/ETH)
        vm.prank(user1);
        protocol.borrow(10_000 * 1e18);
        
        (uint256 collateral, uint256 debt, uint256 hf) = protocol.getPosition(user1);
        assertEq(collateral, 10 ether);
        assertEq(debt, 10_000 * 1e18);
        assertGt(hf, 100); // Health factor should be > 100%
    }
    
    function testCannotOverBorrow() public {
        vm.prank(user1);
        protocol.deposit{value: 10 ether}();
        
        // Try to borrow more than allowed (>66% of collateral value)
        vm.prank(user1);
        vm.expectRevert("Would become undercollateralized");
        protocol.borrow(15_000 * 1e18);
    }
    
    function testLiquidation() public {
        // User1 deposits and borrows
        vm.prank(user1);
        protocol.deposit{value: 10 ether}();
        
        vm.prank(user1);
        protocol.borrow(10_000 * 1e18);
        
        // Price drops, making position undercollateralized
        protocol.setEthPrice(1300 * 1e18); // ETH drops to $1300
        
        // Check position is liquidatable
        assertTrue(protocol.isLiquidatable(user1));
        
        // Liquidator approves and liquidates
        vm.startPrank(liquidator);
        stablecoin.approve(address(protocol), 10_000 * 1e18);
        
        uint256 liquidatorEthBefore = liquidator.balance;
        protocol.liquidate(user1, 10_000 * 1e18);
        uint256 liquidatorEthAfter = liquidator.balance;
        
        // Liquidator should receive collateral + bonus
        assertGt(liquidatorEthAfter, liquidatorEthBefore);
        vm.stopPrank();
        
        // User1's debt should be reduced
        (, uint256 debtAfter, ) = protocol.getPosition(user1);
        assertEq(debtAfter, 0);
    }
    
    function testCannotLiquidateHealthyPosition() public {
        vm.prank(user1);
        protocol.deposit{value: 10 ether}();
        
        vm.prank(user1);
        protocol.borrow(5_000 * 1e18);
        
        // Position is healthy
        assertFalse(protocol.isLiquidatable(user1));
        
        // Try to liquidate
        vm.startPrank(liquidator);
        stablecoin.approve(address(protocol), 5_000 * 1e18);
        vm.expectRevert("Position is healthy");
        protocol.liquidate(user1, 5_000 * 1e18);
        vm.stopPrank();
    }
    
    function testRepay() public {
        // Setup position
        vm.prank(user1);
        protocol.deposit{value: 10 ether}();
        
        vm.prank(user1);
        protocol.borrow(5_000 * 1e18);
        
        // Repay
        vm.startPrank(user1);
        stablecoin.approve(address(protocol), 5_000 * 1e18);
        protocol.repay(5_000 * 1e18);
        vm.stopPrank();
        
        (, uint256 debt, ) = protocol.getPosition(user1);
        assertEq(debt, 0);
    }
    
    function testWithdraw() public {
        vm.prank(user1);
        protocol.deposit{value: 10 ether}();
        
        uint256 balanceBefore = user1.balance;
        
        vm.prank(user1);
        protocol.withdraw(5 ether);
        
        uint256 balanceAfter = user1.balance;
        assertEq(balanceAfter - balanceBefore, 5 ether);
        
        (uint256 collateral, , ) = protocol.getPosition(user1);
        assertEq(collateral, 5 ether);
    }
}



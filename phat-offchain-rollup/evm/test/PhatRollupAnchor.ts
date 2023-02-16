import { loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import { expect } from "chai";
import { ethers } from "hardhat";

describe("RollupAnchor", function () {
  async function deployFixture() {
    // Contracts are deployed using the first signer/account by default
    const [owner, submitter] = await ethers.getSigners();

    const Anchor = await ethers.getContractFactory("PhatRollupAnchor");
    const TestReceiver = await ethers.getContractFactory("TestReceiver");
    const receiver = await TestReceiver.deploy();
    const queuePrefix = hex('q');
    const anchor = await Anchor.deploy(submitter.address, receiver.address, queuePrefix);

    return { anchor, receiver, owner, submitter };
  }

  // describe("Deployment", function () {
  //   it("Should set the right unlockTime", async function () {
  //     const { anchor, receiver } = await loadFixture(deployFixture);
  //   });

  //   it("Should get the constant calcuated", async function () {
  //     const { anchor } = await loadFixture(deployFixture);
  //     expect(await anchor.genReceiverSelector()).to.equal("0x43a53d89");
  //   })

  //   it("Should convert bytes to uint256", async function () {
  //     const { anchor } = await loadFixture(deployFixture);
  //     expect(await anchor.testConvert('0x')).to.equal('0');
  //     expect(await anchor.testConvert('0x0000000000000000000000000000000000000000000000000000000000000000')).to.equal('0');
  //     expect(await anchor.testConvert('0x0000000000000000000000000000000000000000000000000000000000000001')).to.equal('1');
  //   });
  // });

  describe("Rollup", function () {
    it("Should not forward from random sender", async function () {
      const { anchor, owner } = await loadFixture(deployFixture);
      await expect(
        anchor.connect(owner).rollupU256CondEq(
          // cond
          [], [],
          // updates
          [], [],
          // actions
          ['0x00DEADBEEF'],
        )
      ).to.be.revertedWith('bad submitter');
    });

    it("Should not allow invalid input arrays", async function () {
      const { anchor, submitter } = await loadFixture(deployFixture);

      await expect(
        anchor.connect(submitter).rollupU256CondEq(
          // cond
          ['0x01'], [],
          // updates
          [], [],
          // actions
          ['0x00DEADBEEF'],
        )
      ).to.be.revertedWith('bad cond len');

      await expect(
        anchor.connect(submitter).rollupU256CondEq(
          // cond
          [], [],
          // updates
          ['0x'], [],
          // actions
          ['0x00DEADBEEF'],
        )
      ).to.be.revertedWith('bad update len');
    });

    it("Should forward actions", async function () {
      const { anchor, receiver, submitter } = await loadFixture(deployFixture);

      await expect(
        anchor.connect(submitter).rollupU256CondEq(
          // cond
          ['0x01'],
          [encodeUint32(0)],
          // updates
          ['0x01'],
          [encodeUint32(1)],
          // actions (0x01 - callback; 0xdeadbeef - data)
          ['0x00DEADBEEF'],
        )
      ).not.to.be.reverted;

      expect(await receiver.getRecvLength()).to.be.equals('1');
      expect(await receiver.getRecv(0)).to.be.eql([anchor.address, '0xdeadbeef']);
      expect(await anchor.getStorage('0x01')).to.be.equals(encodeUint32(1));
    });
  });

  describe("OptimisticLock", function () {
    it("Should reject conflicting transaction", async function () {
      const { anchor, receiver, submitter } = await loadFixture(deployFixture);
      // Rollup from v0 to v1
      await expect(
        anchor.connect(submitter).rollupU256CondEq(
          // cond
          ['0x01'],
          [encodeUint32(0)],
          // updates
          ['0x01'],
          [encodeUint32(1)],
          // actions
          ['0x00DEADBEEF'],
        )
      ).not.to.be.reverted;
      expect(await anchor.getStorage('0x01')).to.be.equals(encodeUint32(1));
      // Rollup to v1 again
      await expect(
        anchor.connect(submitter).rollupU256CondEq(
          // cond
          ['0x01'],
          [encodeUint32(0)],
          // updates
          ['0x01'],
          [encodeUint32(1)],
          // actions
          ['0x00DEADBEEF'],
        )
      ).to.be.revertedWith('cond not met');
    });
  });


  describe("Rollup", function () {
    it("Can process requests", async function () {
        const { anchor, receiver, owner, submitter } = await loadFixture(deployFixture);
        const pushTx = await anchor.connect(owner).pushMessage('0xdecaffee');
        await expect(pushTx).not.to.be.reverted;
        await expect(pushTx).to
            .emit(anchor, 'MessageQueued')
            .withArgs(0, '0xdecaffee');
        // Simulate a rollup
        const rollupTx = await anchor.connect(submitter).rollupU256CondEq(
                // cond (global=1)
                ['0x00'],
                [encodeUint32(0)],
                // updates (global=2)
                ['0x00'],
                [encodeUint32(1)],
                // actions 
                [
                    // Callback: req 00 responded with 0xDEADBEEF
                    ethers.utils.hexConcat(['0x00', encodeUint32(0), '0xDEADBEEF']),
                    // Custom: queue processed to 1
                    ethers.utils.hexConcat(['0x01', encodeUint32(1)]),
                ],
            )
        await expect(rollupTx).not.to.be.reverted;
        await expect(rollupTx).to
            .emit(anchor, 'MessageProcessedTo')
            .withArgs(1);

        // Check queue processed
        expect(await receiver.getRecvLength()).to.be.equals('1');
        expect(await receiver.getRecv(0)).to.be.eql([anchor.address, '0x0000000000000000000000000000000000000000000000000000000000000000deadbeef']);
        // end
        expect(await anchor.queueGetUint(hex('_tail'))).to.be.equals(1);
        // start
        expect(await anchor.queueGetUint(hex('_head'))).to.be.equals(1);
    })
  });
});


function abiEncode(type: string, value: any) {
  return ethers.utils.defaultAbiCoder.encode([type], [value]);
}
function encodeUint32(v: number) {
  return abiEncode('uint32', v);
}
function hex(str: string): string {
  return ethers.utils.hexlify(ethers.utils.toUtf8Bytes(str));
}

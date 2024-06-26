// Copyright 2022 Cartesi Pte. Ltd.

// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy
// of the license at http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.

import React, { useEffect, useState } from "react";
import { ethers } from "ethers";
import { useRollups } from "./useRollups";
import { useWallets } from "@web3-onboard/react";
import {
  Tabs,
  TabList,
  TabPanels,
  TabPanel,
  Tab,
  Card,
  useColorMode,
  Input,
} from "@chakra-ui/react";
import { Button, Box } from "@chakra-ui/react";
import { Stack } from "@chakra-ui/react";
import { Accordion } from "@chakra-ui/react";
import { Text } from "@chakra-ui/react";
import { Vouchers } from "./Vouchers";
import { EtherInput } from "./components/EtherInput";
import { decode as bs58decode } from '@web3pack/base58-check';

interface IInputPropos {
  dappAddress: string;
}

export const Transfers: React.FC<IInputPropos> = (propos) => {
  const rollups = useRollups(propos.dappAddress);
  const [connectedWallet] = useWallets();
  const provider = new ethers.providers.Web3Provider(connectedWallet.provider);
  const { colorMode } = useColorMode();

  const depositEtherToPortal = async (amount: string, destAddress: string) => {
    console.log(`Depositing ${amount} to ${destAddress}`);
    try {
      if (rollups && provider) {
        // parse the t-address into bytes we can send to the contract
        let address_bytes = bs58decode(destAddress).subarray(2); // skip the first two bytes as they carry no information
        const data = ethers.utils.arrayify(address_bytes);
        const txOverrides = {
          value: ethers.utils.parseEther(amount),
        };
        console.log("Ether to deposit: ", txOverrides);
        console.log("Destination address: ", address_bytes);

        // const tx = await ...
        rollups.etherPortalContract.depositEther(
          propos.dappAddress,
          data,
          txOverrides
        );
      }
    } catch (e) {
      console.log(`${e}`);
    }
  };

  const [etherAmount, setEtherAmount] = useState<string>("0");
  const [destAddress, setDestAddress] = useState<string>("t1");

  const [withdrawAmount, setWithdrawAmount] = useState<string>("0");
  const [withdrawAddress, setWithdrawAddress] = useState<string>(connectedWallet.accounts[0].address);
  const [withdrawCommand, setWithdrawCommand] = useState<string>("");

  useEffect(() => {
    setWithdrawCommand(`send ${rollups?.rollupExitAddress} ${ethers.utils.parseEther(withdrawAmount).div(10000000000)} ${withdrawAddress.replace('0x', '')}`)
  }, [withdrawAddress, withdrawAmount, rollups])

  return (
    <Card
      colorScheme="blackAlpha"
      marginY={"28px"}
      rounded={24}
      borderWidth={"1px"}
      borderColor={"#e0e2eb"}
    >
      <Tabs
        colorScheme="blackAlpha"
        isFitted
        variant="soft-rounded"
        borderRadius={2}
        size="lg"
        align="center"
      >
        <TabList
          margin={5}
          rounded={8}
          bg={colorMode === "light" ? "#e0e2eb" : "#bcbfcd"}
        >
          <Tab
            margin={1}
            padding={2}
            borderRadius={8}
            _selected={{
              bg: colorMode === "light" ? "#f2f3f8" : "#232634",
            }}
            color={colorMode === "light" ? "black" : "white"}
          >
            Deposit
          </Tab>
          {/* <Tab
            margin={1}
            padding={2}
            borderRadius={8}
            _selected={{
              bg: colorMode === "light" ? "#f2f3f8" : "#232634",
            }}
            color={colorMode === "light" ? "black" : "white"}
          >
            Transact
          </Tab> */}
          <Tab
            margin={1}
            padding={2}
            borderRadius={8}
            _selected={{
              bg: colorMode === "light" ? "#f2f3f8" : "#232634",
            }}
            color={colorMode === "light" ? "black" : "white"}
          >
            Withdraw
          </Tab>
        </TabList>
        <Box p={4} display="flex">
          <TabPanels>
            <TabPanel>
              <Text fontSize="sm" color="grey">
                Deposit Eth to bridge it to CarteZcash
              </Text>
              <br />
              <Stack>
                <label>Amount (Eth)</label>
                <EtherInput
                  onChange={(value: string) => setEtherAmount(value)}
                  value={etherAmount}
                />
                <label>Destination Zcash Address</label>
                <Input
                  value={destAddress}
                  onChange={(e) => setDestAddress(e.target.value)}
                />
                <Button
                  size="sm"
                  onClick={() => {
                    depositEtherToPortal(etherAmount, destAddress);
                  }}
                  disabled={!rollups}
                >
                  Deposit
                </Button>
              </Stack>
              <br />
            </TabPanel>

            {/* <TabPanel> Skip this for now. It was part of the earlier demo but now we can sent transactions directly from the wallet
              <Text fontSize="sm" color="grey">
                Send ZCash transactions to have them executed on the rollup
              </Text>
              <Stack>
                <label>Transaction Hex</label>
                <Input
                  value={transactionHex}
                  height={100}
                  onChange={(e) => setTransactionHex(e.target.value)}
                ></Input>
                <Button
                  size="sm"
                  onClick={() => {
                    sendTransaction(transactionHex);
                  }}
                  disabled={!rollups}
                >
                  Transact
                </Button>
              </Stack>
            </TabPanel> */}

            <TabPanel>
              <Accordion defaultIndex={[0]} allowMultiple>
                <Text fontSize="large" color="grey">
                  To withdraw set the parameters then execute the resulting command in Zingo-cli
                </Text>
                <Stack>
                <label>Amount (Eth)</label>
                <EtherInput
                  onChange={(value: string) => setWithdrawAmount(value)}
                  value={withdrawAmount}
                />
                <label>Withdrawal Eth Address</label>
                <Input value={withdrawAddress} onChange={(e) => setWithdrawAddress(e.target.value)}></Input>
                <label>Zingo withdraw command</label>
                <Input value={withdrawCommand} disabled={true}></Input>
                </Stack>
                <br />
                <Text fontSize="large" color="grey">
                  Once processed a voucher will appear here to claim the funds
                </Text>
                <Vouchers dappAddress={propos.dappAddress} />
              </Accordion>
            </TabPanel>
          </TabPanels>
        </Box>
      </Tabs>
    </Card>
  );
};

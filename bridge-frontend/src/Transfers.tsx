// Copyright 2022 Cartesi Pte. Ltd.

// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy
// of the license at http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.

import React, { useState } from "react";
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
} from "@chakra-ui/react";
import { Button, Box } from "@chakra-ui/react";
import { Input, Stack } from "@chakra-ui/react";
import { Accordion } from "@chakra-ui/react";
import { Text } from "@chakra-ui/react";
import { Vouchers } from "./Vouchers";
import { EtherInput } from "./components/EtherInput";
import { ZCashTaddressInput } from "./components/ZCashTaddressInput";
import bs58 from "bs58";

interface IInputPropos {
  dappAddress: string;
}

export const Transfers: React.FC<IInputPropos> = (propos) => {
  const rollups = useRollups(propos.dappAddress);
  const [connectedWallet] = useWallets();
  const provider = new ethers.providers.Web3Provider(connectedWallet.provider);
  const { colorMode } = useColorMode();

  const sendAddress = async () => {
    if (rollups) {
      try {
        await rollups.relayContract.relayDAppAddress(propos.dappAddress);
        // setDappRelayedAddress(true);
      } catch (e) {
        console.log(`${e}`);
      }
    }
  };

  const depositEtherToPortal = async (amount: string, destAddress: string) => {
    try {
      if (rollups && provider) {
        // parse the t-address into bytes we can send to the contract
        let address_bytes = bs58.decode(destAddress);
        const data = ethers.utils.arrayify(address_bytes);
        const txOverrides = {
          value: ethers.utils.parseEther(amount),
        };
        console.log("Ether to deposit: ", txOverrides);

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

  const sendTransaction = async (transactionHex: string) => {
    try {
      if (rollups && provider) {
        const input = ethers.utils.arrayify(transactionHex);

        rollups.inputContract.addInput(propos.dappAddress, input);
      }
    } catch (e) {
      console.log(`${e}`);
    }
  };

  const [etherAmount, setEtherAmount] = useState<string>("");
  const [destAddress, setDestAddress] = useState<string>("t1");

  const [transactionHex, setTransactionHex] = useState<string>("");

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
                <ZCashTaddressInput
                  value={destAddress}
                  onChange={(e: string) => setDestAddress(e)}
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
                  To withdraw send funds to the exit address{" "}
                  {process.env.REACT_APP_EXIT_ADDRESS} on the L2 then execute
                  the resulting voucher here
                </Text>
                <br />
                <Vouchers dappAddress={propos.dappAddress} />
              </Accordion>
            </TabPanel>
          </TabPanels>
        </Box>
      </Tabs>
    </Card>
  );
};

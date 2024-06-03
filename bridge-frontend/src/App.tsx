// Copyright 2022 Cartesi Pte. Ltd.

// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy
// of the license at http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.

import { FC, useEffect } from "react";
import injectedModule from "@web3-onboard/injected-wallets";
import { init, useConnectWallet, useSetChain } from "@web3-onboard/react";
import metamaskSDK from "@web3-onboard/metamask";
import { useState } from "react";

import { GraphQLProvider } from "./GraphQL";
import { Transfers } from "./Transfers";
import configFile from "./config.json";
import "./App.css";
import {
  Box,
  Stack,
  SimpleGrid,
  Button,
  Heading,
  Text,
  Image,
} from "@chakra-ui/react";
import banner from "./banner.png";
import Header from "./Header";

const config: any = configFile;

// initialize the module with options
const metamaskSDKWallet = metamaskSDK({
  options: {
    extensionOnly: false,
    dappMetadata: {
      name: "CarteZcash Bridge",
    },
  },
});

const injected: any = injectedModule();

init({
  wallets: [injected, metamaskSDKWallet],
  chains: Object.entries(config).map(([k, v]: [string, any], i) => ({
    id: k,
    token: v.token,
    label: v.label,
    rpcUrl: v.rpcUrl,
  })),
  appMetadata: {
    name: "CarteZcash Testnet",
    icon: "<svg><svg/>",
    description: "Cartesi shielded pool using ZCash",
    recommendedInjectedWallets: [
      { name: "MetaMask", url: "https://metamask.io" },
    ],
  },
});

const App: FC = () => {
  const [{ connectedChain }] = useSetChain();

  const [{ wallet, connecting }, connect] = useConnectWallet();
  const [dappAddress, setDappAddress] = useState<string>("");

  useEffect(() => {
    const switchChain = async (hexChainId: string) => {
      try {
        const result = await wallet?.provider.request({
          method: "wallet_switchEthereumChain",
          params: [{ chainId: hexChainId }], // chainId must be in hexadecimal numbers
        });
        console.log(`result: `, result);
      } catch (err: unknown) {
        console.log(typeof err);
        console.log(err);
      }
    };

    if (connectedChain) {
      if (connectedChain.id !== "0xaa36a7") {
        switchChain("0xaa36a7");
      } else {
        setDappAddress(config[connectedChain.id].DAppAddress);
      }
    }
  }, [connectedChain, wallet]);

  return (
    <>
      <Header dappAddress={dappAddress} setDappAddress={setDappAddress} />
      <SimpleGrid columns={1} marginX={"30%"} alignContent={"center"}>
        {!wallet && (
          <Box mt="28px" alignContent="center">
            <Stack>
              <Heading>CarteZcash Bridge</Heading>
              <Text>
                Connect a wallet to deposit or withdraw Eth from the rollup
              </Text>
              <Image src={banner} alt="Banner" />
              <Button
                onClick={() => connect()}
                marginY={"100px"}
                disabled={connecting}
              >
                {connecting ? "Connecting" : "Connect"}
              </Button>
            </Stack>
          </Box>
        )}
        <GraphQLProvider>
          <Transfers dappAddress={dappAddress} />
        </GraphQLProvider>
      </SimpleGrid>
    </>
  );
};

export default App;

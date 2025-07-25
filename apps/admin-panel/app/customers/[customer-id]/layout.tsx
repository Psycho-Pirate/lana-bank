"use client"

import { gql } from "@apollo/client"
import { use, useEffect } from "react"
import { useTranslations } from "next-intl"

import { Tabs, TabsList, TabsTrigger, TabsContent } from "@lana/web/ui/tab"
import { ScrollArea, ScrollBar } from "@lana/web/ui/scroll-area"

import { CustomerDetailsCard } from "./details"
import { KycStatus } from "./kyc-status"
import { CustomerAccountBalances } from "./balances"

import { useTabNavigation } from "@/hooks/use-tab-navigation"
import {
  Customer as CustomerType,
  useGetCustomerBasicDetailsQuery,
} from "@/lib/graphql/generated"
import { useCreateContext } from "@/app/create"
import { useBreadcrumb } from "@/app/breadcrumb-provider"
import { DetailsPageSkeleton } from "@/components/details-page-skeleton"

gql`
  query GetCustomerBasicDetails($id: PublicId!) {
    customerByPublicId(id: $id) {
      id
      customerId
      email
      telegramId
      status
      level
      customerType
      createdAt
      publicId
      depositAccount {
        id
        publicId
        depositAccountId
        balance {
          settled
          pending
        }
      }
    }
  }
`

export default function CustomerLayout({
  children,
  params,
}: {
  children: React.ReactNode
  params: Promise<{ "customer-id": string }>
}) {
  const t = useTranslations("Customers.CustomerDetails.layout")
  const navTranslations = useTranslations("Sidebar.navItems")

  const TABS = [
    { id: "1", url: "/", tabLabel: t("tabs.transactions") },
    { id: "2", url: "/credit-facilities", tabLabel: t("tabs.creditFacilities") },
    { id: "4", url: "/documents", tabLabel: t("tabs.documents") },
  ]

  const { "customer-id": customerId } = use(params)
  const { currentTab, handleTabChange } = useTabNavigation(TABS, customerId)

  const { setCustomLinks, resetToDefault } = useBreadcrumb()

  const { setCustomer } = useCreateContext()
  const { data, loading, error } = useGetCustomerBasicDetailsQuery({
    variables: { id: customerId },
  })

  useEffect(() => {
    if (data?.customerByPublicId) setCustomer(data.customerByPublicId as CustomerType)
    return () => setCustomer(null)
  }, [data?.customerByPublicId, setCustomer])

  useEffect(() => {
    if (data?.customerByPublicId) {
      const currentTabData = TABS.find((tab) => tab.url === currentTab)
      setCustomLinks([
        { title: navTranslations("customers"), href: "/customers" },
        { title: data.customerByPublicId.email, href: `/customers/${customerId}` },
        ...(currentTabData?.url === "/"
          ? []
          : [{ title: currentTabData?.tabLabel ?? "", isCurrentPage: true as const }]),
      ])
    }
    return () => {
      resetToDefault()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data?.customerByPublicId, currentTab])

  if (loading && !data) return <DetailsPageSkeleton detailItems={3} tabs={6} />
  if (error) return <div className="text-destructive">{t("errors.error")}</div>
  if (!data || !data.customerByPublicId) return null

  return (
    <main className="max-w-7xl m-auto">
      <CustomerDetailsCard customer={data.customerByPublicId} />
      <div className="flex flex-col md:flex-row w-full gap-2 my-2">
        <KycStatus customerId={data.customerByPublicId.customerId} />
        {data.customerByPublicId.depositAccount && (
          <CustomerAccountBalances
            balance={data.customerByPublicId.depositAccount.balance}
            publicId={data.customerByPublicId.depositAccount.publicId}
          />
        )}
      </div>
      <Tabs
        defaultValue={TABS[0].url}
        value={currentTab}
        onValueChange={handleTabChange}
        className="mt-2"
      >
        <ScrollArea>
          <div className="relative h-10">
            <TabsList className="flex absolute h-10">
              {TABS.map((tab) => (
                <TabsTrigger key={tab.id} value={tab.url} id={`tab-${tab.id}`}>
                  {tab.tabLabel}
                </TabsTrigger>
              ))}
            </TabsList>
          </div>
          <ScrollBar orientation="horizontal" />
        </ScrollArea>
        {TABS.map((tab) => (
          <TabsContent key={tab.id} value={tab.url}>
            {children}
          </TabsContent>
        ))}
      </Tabs>
    </main>
  )
}

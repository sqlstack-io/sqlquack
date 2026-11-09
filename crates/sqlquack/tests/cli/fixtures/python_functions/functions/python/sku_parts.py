from sqlquack import udf


@udf(arguments={"sku": "VARCHAR"}, returns="VARIANT")
def main(sku):
    return None

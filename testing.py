import rapid_pdf

items = rapid_pdf.extract_text_from_pdf("test.pdf")

print(items)

for item in items:
    # print(item.x)
    print(item.text, item.x, item.y, item.font_size)
-dontobfuscate
-keepattributes SourceFile, LineNumberTable, Exception, *Annotation*, InnerClasses, EnclosingMethod, Signature

-allowaccessmodification
-dontskipnonpubliclibraryclasses
-dontskipnonpubliclibraryclassmembers

-keep,allowoptimization class MainKt {
	public static void main(java.lang.String[]);
}
